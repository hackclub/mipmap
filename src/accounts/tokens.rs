use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, Generate, KeyInit},
};
use anyhow::anyhow;
use sqlx::{PgExecutor, PgPool};

#[derive(Clone, Copy)]
pub enum Provider {
    Hca,
    Hackatime,
}

impl Provider {
    fn name(self) -> &'static str {
        match self {
            Provider::Hca => "hca",
            Provider::Hackatime => "hackatime",
        }
    }
}

fn cipher() -> anyhow::Result<Aes256Gcm> {
    let key = hex::decode(std::env::var("TOKEN_ENCRYPTION_KEY")?)?;
    Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow!("TOKEN_ENCRYPTION_KEY must be 32 bytes of hex"))
}

pub async fn store(
    executor: impl PgExecutor<'_>,
    user_id: i32,
    provider: Provider,
    token: &str,
) -> anyhow::Result<()> {
    let nonce = Nonce::generate();
    let ciphertext = cipher()?
        .encrypt(&nonce, token.as_bytes())
        .map_err(|_| anyhow!("token encryption failed"))?;
    sqlx::query(
        "INSERT INTO auth_token (user_id, provider, ciphertext, nonce) VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, provider) DO UPDATE SET ciphertext = $3, nonce = $4",
    )
    .bind(user_id)
    .bind(provider.name())
    .bind(ciphertext)
    .bind(nonce.as_slice())
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn load(
    pool: &PgPool,
    user_id: i32,
    provider: Provider,
) -> anyhow::Result<Option<String>> {
    let Some((ciphertext, nonce)) = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
        "SELECT ciphertext, nonce FROM auth_token WHERE user_id = $1 AND provider = $2",
    )
    .bind(user_id)
    .bind(provider.name())
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let nonce = Nonce::try_from(nonce.as_slice()).map_err(|_| anyhow!("bad nonce"))?;
    let token = cipher()?
        .decrypt(&nonce, ciphertext.as_slice())
        .map_err(|_| anyhow!("token decryption failed"))?;
    Ok(Some(String::from_utf8(token)?))
}
