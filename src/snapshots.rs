use actix_web::{HttpRequest, HttpResponse, get, post, web};
use hc_helper::get_auth_token;
use serde::Serialize;

const EVERYONE_CAPTIONS: [&str; 24] = [
    "Got the PCB routed!",
    "First light on the raytracer",
    "Soldered the last header pin",
    "Finally got the OLED displaying text",
    "Our robot arm took its first steps",
    "Tested the new keyboard layout",
    "Enclosure came off the printer clean",
    "Shaders are compiling at last",
    "Wired up the sensor array",
    "Demo day prep is going well",
    "The synth plays its first chord",
    "Mapped out the whole level in Tiled",
    "Fixed the bug that ate my weekend",
    "Weather station is online",
    "Prototype number three is alive",
    "Laser-cut the case today",
    "Drone passed its first hover test",
    "Added dark mode to the dashboard",
    "Flashed the firmware without errors",
    "Hand-wired the whole macropad",
    "Particle effects look great",
    "Calibrated the stepper motors",
    "Plotted my first pen drawing",
    "Repo is finally public",
];

const MY_CAPTIONS: [&str; 5] = [
    "Sketched out the first design",
    "Breadboard prototype works",
    "Ordered the PCBs",
    "Case design is done",
    "Everything fits together",
];

const AUTHORS: [&str; 6] = ["Ava", "Kai", "Mina", "Theo", "Priya", "Jonah"];

#[derive(Serialize)]
struct Snapshot {
    id: u32,
    image: String,
    caption: String,
    author: String,
}

#[derive(Serialize)]
struct Status {
    tracked_seconds: u32,
    required_seconds: u32,
    sent_today: bool,
}

fn dummy(kind: &str, captions: &[&str], authors: &[&str]) -> Vec<Snapshot> {
    (1..=captions.len() as u32)
        .rev()
        .map(|n| Snapshot {
            id: n,
            image: format!("https://picsum.photos/seed/mipmap-{kind}-{n}/800/600"),
            caption: captions[n as usize - 1].to_string(),
            author: authors[n as usize % authors.len()].to_string(),
        })
        .collect()
}

#[get("/snapshots")]
pub async fn list_snapshots(req: HttpRequest) -> HttpResponse {
    let _token = get_auth_token!(req);
    HttpResponse::Ok().json(dummy("everyone", &EVERYONE_CAPTIONS, &AUTHORS))
}

#[get("/snapshots/mine")]
pub async fn my_snapshots(req: HttpRequest) -> HttpResponse {
    let _token = get_auth_token!(req);
    HttpResponse::Ok().json(dummy("mine", &MY_CAPTIONS, &["Orpheus"]))
}

#[get("/snapshots/status")]
pub async fn status(req: HttpRequest) -> HttpResponse {
    let _token = get_auth_token!(req);
    HttpResponse::Ok().json(Status {
        tracked_seconds: 4140,
        required_seconds: 900,
        sent_today: false,
    })
}

#[post("/snapshots")]
pub async fn create_snapshot(req: HttpRequest, _body: web::Bytes) -> HttpResponse {
    let _token = get_auth_token!(req);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, cookie::Cookie, http::StatusCode, test};
    use serde_json::{Value, json};

    #[actix_web::test]
    async fn snapshots_flow() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api")
                    .service(list_snapshots)
                    .service(my_snapshots)
                    .service(status)
                    .service(create_snapshot),
            ),
        )
        .await;

        let get = |uri: &str| {
            test::TestRequest::get()
                .uri(uri)
                .cookie(Cookie::new("auth-token", "snapshots-test-user"))
                .to_request()
        };

        let res = test::call_service(&app, get("/api/snapshots")).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(body.as_array().unwrap().len(), 24);
        assert_eq!(body[0]["id"], 24);
        assert_eq!(
            body[0]["image"],
            "https://picsum.photos/seed/mipmap-everyone-24/800/600"
        );
        assert_eq!(body[23]["id"], 1);

        let res = test::call_service(&app, get("/api/snapshots/mine")).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(body.as_array().unwrap().len(), 5);
        assert_eq!(
            body[0]["image"],
            "https://picsum.photos/seed/mipmap-mine-5/800/600"
        );

        let res = test::call_service(&app, get("/api/snapshots/status")).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = test::read_body_json(res).await;
        assert_eq!(
            body,
            json!({ "tracked_seconds": 4140, "required_seconds": 900, "sent_today": false })
        );
    }
}
