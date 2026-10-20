use crate::{
    accounts::{
        hackatime::{self, Totals},
        session::User,
    },
    admin, dashboard, env,
    error::{ApiError, reject, reject_with},
    projects::{self, Project},
    uploads,
};
use actix_multipart::form::{MultipartForm, bytes::Bytes, text::Text};
use actix_web::{HttpRequest, HttpResponse, get, http::StatusCode, post, web};
use rustrict::{Censor, CensorStr, Trie, Type};
use serde::Serialize;
use sqlx::{PgExecutor, PgPool, types::Json};
use std::{collections::BTreeSet, sync::LazyLock};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

const REQUIRED_SECONDS: i64 = 15 * 60;
const FEED_SIZE: i64 = 24;
const CAPTION_LIMIT: usize = 500;
const DISPLAY_NAME_LIMIT: usize = 40;
const ALREADY_SENT: &str = "You've already sent a Snapshot today";
const NAME_TAKEN: &str = "Someone else already uses that name";
const MAX_MARKS: usize = 3;
const RESERVED_NAMES: [&str; 2] = ["hackclub", "mipmap"];
const RESERVED_WORDS: [&str; 4] = ["admin", "staff", "moderator", "official"];
const MINORS: &str = "sexual/minors";

#[derive(Serialize, sqlx::FromRow)]
struct Snapshot {
    id: i32,
    image: String,
    caption: String,
    author: String,
}

#[derive(Serialize)]
struct Status {
    tracked_seconds: i64,
    required_seconds: i64,
    sent_today: bool,
    multiplier: f64,
}

pub fn multiplier(shared: i64) -> f64 {
    (100 + shared.clamp(0, 32)) as f64 / 100.0
}

#[derive(Serialize)]
pub struct Overview {
    everyone: Vec<Snapshot>,
    mine: Vec<Snapshot>,
    status: Status,
}

pub async fn overview(
    pool: &PgPool,
    user_id: i32,
    current: Option<&Totals>,
    projects: &[Project],
) -> sqlx::Result<Overview> {
    Ok(Overview {
        status: status(pool, user_id, current, projects).await?,
        everyone: everyone(pool, user_id).await?,
        mine: mine(pool, user_id).await?,
    })
}

async fn everyone(pool: &PgPool, user_id: i32) -> sqlx::Result<Vec<Snapshot>> {
    sqlx::query_as(
        "SELECT snapshot.id,
         CASE WHEN held_image IS NULL THEN image_url ELSE '/api/snapshots/' || snapshot.id || '/image' END AS image,
         caption, display_name AS author
         FROM snapshot JOIN \"user\" ON \"user\".id = snapshot.user_id
         WHERE snapshot.deleted_at IS NULL AND \"user\".deleted_at IS NULL AND NOT \"user\".is_banned
         AND snapshot.shadowbanned_at IS NULL AND (snapshot.held_image IS NULL OR snapshot.user_id = $2)
         ORDER BY snapshot.created_at DESC LIMIT $1",
    )
    .bind(FEED_SIZE)
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn mine(pool: &PgPool, user_id: i32) -> sqlx::Result<Vec<Snapshot>> {
    sqlx::query_as(
        "SELECT id, CASE WHEN held_image IS NULL THEN image_url ELSE '/api/snapshots/' || id || '/image' END AS image,
         caption, display_name AS author FROM snapshot
         WHERE user_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn status(
    pool: &PgPool,
    user_id: i32,
    current: Option<&Totals>,
    projects: &[Project],
) -> sqlx::Result<Status> {
    let (last, baseline, sent_today, shared): (Option<Json<Totals>>, Option<Json<Totals>>, bool, i64) =
        sqlx::query_as(
            "SELECT
                 (SELECT hackatime_totals FROM snapshot
                  WHERE user_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1),
                 hackatime_baseline,
                 EXISTS (SELECT 1 FROM snapshot
                         WHERE user_id = $1 AND deleted_at IS NULL
                         AND created_at > now() - interval '16 hours'),
                 (SELECT count(*) FROM snapshot WHERE user_id = $1 AND deleted_at IS NULL)
             FROM \"user\" WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    let reference = last.or(baseline).map(|json| json.0).unwrap_or_default();
    let linked: BTreeSet<&str> = projects
        .iter()
        .flat_map(|project| project.hackatime.iter().map(String::as_str))
        .collect();
    let tracked_seconds = current.map_or(0, |current| {
        let linked: i64 = linked
            .iter()
            .map(|name| {
                let now = current.get(*name).copied().unwrap_or(0);
                let then = reference.get(*name).copied().unwrap_or(0);
                (now - then).max(0)
            })
            .sum();
        let overall = current.values().sum::<i64>() - reference.values().sum::<i64>();
        linked.min(overall.max(0))
    });
    Ok(Status {
        tracked_seconds,
        required_seconds: REQUIRED_SECONDS,
        sent_today,
        multiplier: multiplier(shared),
    })
}

pub async fn ready(pool: &PgPool, user_id: i32, current: &Totals, projects: &[Project]) -> sqlx::Result<bool> {
    let status = status(pool, user_id, Some(current), projects).await?;
    Ok(!status.sent_today && status.tracked_seconds >= status.required_seconds)
}

static NAMES: LazyLock<Trie> = LazyLock::new(|| {
    let mut trie = Trie::default();
    for word in RESERVED_NAMES {
        trie.set(word, Type::OFFENSIVE & Type::SEVERE);
    }
    for word in RESERVED_WORDS {
        trie.set(&format!(" {word} "), Type::OFFENSIVE & Type::SEVERE);
        trie.set(&format!(" {word}s "), Type::OFFENSIVE & Type::SEVERE);
    }
    trie
});

fn name_allowed(name: &str) -> bool {
    let spaced: String = name
        .chars()
        .map(|c| if matches!(c, '_' | '-' | '.' | ',' | '~' | ':' | '/' | '\\' | '=') { ' ' } else { c })
        .collect();
    Censor::from_str(&spaced)
        .with_trie(&NAMES)
        .analyze()
        .isnt(Type::INAPPROPRIATE)
}

fn invisible(c: char) -> bool {
    (c.is_whitespace() && c != ' ')
        || c.is_control()
        || c.general_category() == GeneralCategory::Format
        || matches!(
            c as u32,
            0xAD | 0x34F
                | 0x61C
                | 0x115F..=0x1160
                | 0x17B4..=0x17B5
                | 0x180B..=0x180F
                | 0x200B..=0x200F
                | 0x202A..=0x202E
                | 0x2060..=0x206F
                | 0x2800
                | 0x3164
                | 0xFE00..=0xFE0F
                | 0xFEFF
                | 0xFFA0
                | 0xFFF0..=0xFFFB
                | 0x1D173..=0x1D17A
                | 0xE0000..=0xE0FFF
        )
}

fn stacked_marks(text: &str) -> bool {
    let mut run = 0;
    text.nfd().any(|c| {
        run = if is_combining_mark(c) { run + 1 } else { 0 };
        run > MAX_MARKS
    })
}

fn name_key(name: &str) -> String {
    let name = name.nfkc().collect::<String>().to_lowercase();
    let skeleton: String = unicode_security::skeleton(&name)
        .collect::<String>()
        .to_lowercase()
        .chars()
        .filter(|&c| !is_combining_mark(c))
        .collect();
    skeleton.replace('i', "l").split_whitespace().collect::<Vec<_>>().join(" ")
}

pub async fn backfill_name_keys(pool: &PgPool) -> sqlx::Result<()> {
    let (ids, keys): (Vec<i32>, Vec<String>) =
        sqlx::query_as::<_, (i32, String, Option<String>)>("SELECT id, display_name, name_key FROM snapshot")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|(id, name, key)| (id, name_key(&name), key))
            .filter(|(_, key, stored)| stored.as_ref() != Some(key))
            .map(|(id, key, _)| (id, key))
            .unzip();
    if ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "UPDATE snapshot SET name_key = t.key
         FROM UNNEST($1::int[], $2::text[]) AS t (id, key) WHERE snapshot.id = t.id",
    )
    .bind(&ids)
    .bind(&keys)
    .execute(pool)
    .await?;
    log::info!("Backfilled the name key of {} snapshots", ids.len());
    Ok(())
}

async fn name_taken(db: impl PgExecutor<'_>, user_id: i32, key: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM snapshot
                        WHERE user_id <> $1 AND deleted_at IS NULL AND name_key = $2)",
    )
    .bind(user_id)
    .bind(key)
    .fetch_one(db)
    .await
}

#[derive(MultipartForm)]
struct SnapshotForm {
    #[multipart(limit = "10MB")]
    image: Bytes,
    caption: Text<String>,
    display_name: Text<String>,
}

#[post("/snapshots")]
pub async fn create_snapshot(
    User(user_id): User,
    pool: web::Data<PgPool>,
    MultipartForm(form): MultipartForm<SnapshotForm>,
) -> Result<HttpResponse, ApiError> {
    let caption = form.caption.trim();
    let display_name = form.display_name.trim();
    if caption.is_empty() || caption.chars().count() > CAPTION_LIMIT {
        return Err(reject(format!("Caption must be 1 to {CAPTION_LIMIT} characters")));
    }
    if display_name.is_empty() || display_name.chars().count() > DISPLAY_NAME_LIMIT {
        return Err(reject(format!(
            "Publicly displayed username must be 1 to {DISPLAY_NAME_LIMIT} characters"
        )));
    }
    if display_name.chars().any(invisible) {
        return Err(reject("Names can't contain invisible characters"));
    }
    if stacked_marks(display_name) || stacked_marks(caption) {
        return Err(reject("Too many accents stacked on one letter"));
    }
    if caption.chars().any(|c| invisible(c) && !matches!(c, '\n' | '\r')) {
        return Err(reject("Captions can't contain invisible characters"));
    }
    let key = name_key(display_name);
    if name_taken(pool.get_ref(), user_id, &key).await? {
        return Err(reject(NAME_TAKEN));
    }
    uploads::ensure_image(&form.image.data)?;

    let Some(hackatime) = hackatime::summaries(&pool, user_id, false).await else {
        return Err(reject("Link your Hackatime account first"));
    };
    let totals = hackatime::totals(&hackatime);
    let projects = projects::list(&pool, user_id).await?;
    let status = status(&pool, user_id, Some(&totals), &projects).await?;
    if status.sent_today {
        return Err(reject(ALREADY_SENT));
    }
    if status.tracked_seconds < status.required_seconds {
        return Err(reject("You haven't tracked enough time yet"));
    }

    let slot = uploads::reserve(user_id)?;
    let mut reasons = Vec::new();
    if !name_allowed(display_name) {
        reasons.push("Name word filter".to_owned());
    }
    if caption.is_inappropriate() {
        reasons.push("Caption word filter".to_owned());
    }
    let text = uploads::text_flags(&[display_name, caption]).await?;
    let text: Vec<String> = ["Name", "Caption"]
        .iter()
        .zip(&text)
        .flat_map(|(label, flags)| flags.iter().map(move |flag| format!("{label}: {flag}")))
        .collect();
    if text.iter().any(|flag| flag.ends_with(MINORS)) {
        reject_minors(&pool, user_id, display_name, caption, "text", &text).await?;
        return Err(reject("This text isn't allowed"));
    }
    reasons.extend(text);
    let webp = uploads::prepare(&slot, &form.image.data).await?;
    let image: Vec<String> = uploads::image_flags(&webp)
        .await?
        .into_iter()
        .map(|flag| format!("Image: {flag}"))
        .collect();
    if image.iter().any(|flag| flag.ends_with(MINORS)) {
        reject_minors(&pool, user_id, display_name, caption, "image", &image).await?;
        return Err(reject("This image isn't allowed"));
    }
    reasons.extend(image);
    let (url, held) = if reasons.is_empty() {
        (uploads::store(webp).await?, None)
    } else {
        (String::new(), Some(webp))
    };
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM \"user\" WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let sent_today: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM snapshot
                        WHERE user_id = $1 AND deleted_at IS NULL
                        AND created_at > now() - interval '16 hours')",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    if sent_today {
        return Err(reject(ALREADY_SENT));
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(&key)
        .execute(&mut *tx)
        .await?;
    if name_taken(&mut *tx, user_id, &key).await? {
        return Err(reject(NAME_TAKEN));
    }
    sqlx::query(
        "INSERT INTO snapshot (user_id, display_name, name_key, caption, image_url, hackatime_totals, held_image, hold_reasons)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(user_id)
    .bind(display_name)
    .bind(&key)
    .bind(caption)
    .bind(url)
    .bind(Json(&totals))
    .bind(held)
    .bind(&reasons)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    dashboard::respond(&pool, user_id).await
}

async fn reject_minors(
    pool: &PgPool,
    user_id: i32,
    display_name: &str,
    caption: &str,
    kind: &str,
    categories: &[String],
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO snapshot_rejection (user_id, display_name, caption, kind, categories)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(display_name)
    .bind(caption)
    .bind(kind)
    .bind(categories)
    .execute(pool)
    .await?;
    Ok(())
}

#[get("/snapshots/{id}/image")]
pub async fn held_image(
    req: HttpRequest,
    User(user_id): User,
    pool: web::Data<PgPool>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    let not_found = || reject_with(StatusCode::NOT_FOUND, "Not Found");
    let (owner, image): (i32, Vec<u8>) = sqlx::query_as(
        "SELECT user_id, held_image FROM snapshot
         WHERE id = $1 AND deleted_at IS NULL AND held_image IS NOT NULL",
    )
    .bind(id.into_inner())
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(not_found)?;
    if owner != user_id && admin::lookup(&req).await?.is_none() {
        return Err(not_found());
    }
    Ok(HttpResponse::Ok()
        .content_type("image/webp")
        .insert_header(("Cache-Control", "private, no-store"))
        .body(image))
}

pub fn spawn_purge(pool: PgPool) {
    actix_web::rt::spawn(async move {
        let mut interval = env::interval_secs("SNAPSHOT_PURGE_INTERVAL_SECS", 3600);
        loop {
            interval.tick().await;
            if let Err(e) = purge(&pool).await {
                log::error!("Snapshot purge failed: {e:#}");
            }
        }
    });
}

async fn purge(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE snapshot SET deleted_at = now(), held_image = NULL
         WHERE held_image IS NOT NULL AND deleted_at IS NULL AND created_at < now() - interval '90 days'",
    )
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM snapshot_rejection WHERE created_at < now() - interval '90 days'")
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        for bad in ["Hack Club", "h4ckclub", "HACK-CLUB official", "xX_admin_Xx", "fuck", "f u c k", "Hаck Clսb", "Admin", "the staff", "Official Bob", "M0derator", "x.staff.x", "ADMINS", "f.u.c.k", "@dmin", "4dm1n"] {
            assert!(!name_allowed(bad), "{bad}");
        }
        for good in ["Orpheus", "Alice", "Scunthorpe", "class act", "Badminton Bob", "Staffan", "Officially Me"] {
            assert!(name_allowed(good), "{good}");
        }
    }

    #[test]
    fn keys() {
        for name in ["Alice", "alice", "ALICE", "Аlice", "AIice", "Ａｌｉｃｅ", "Alice  ", " alice"] {
            assert_eq!(name_key(name), name_key("Alice"), "{name}");
        }
        assert_ne!(name_key("Alice"), name_key("Alicia"));
        assert_ne!(name_key("Alice"), name_key("Ali ce"));
        assert_eq!(name_key("Bob  Smith"), name_key("bob smith"));
        for name in ["Alice\u{200b}", "\u{202e}ecilA", "Al\tice", "Alice\u{feff}", "Al\u{e0041}ice"] {
            assert!(name.chars().any(invisible), "{name:?}");
        }
        assert!(!"Zoë O'Brien".chars().any(invisible));
        for name in ["\u{2800}", "a\u{1bca0}b", "\u{202e}kcuf"] {
            assert!(name.chars().any(invisible), "{name:?}");
        }
        for name in ["Alicé", "Alice\u{301}", "Àlice"] {
            assert_eq!(name_key(name), name_key("Alice"), "{name}");
        }
        assert!(stacked_marks("a\u{300}\u{301}\u{302}\u{303}"));
        assert!(!stacked_marks("Nguyễn Thị Ánh"));
    }
}
