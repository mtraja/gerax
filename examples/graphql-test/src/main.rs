use actix_web::App;
use actix_web::HttpServer;

fn main() {
    actix_web::rt::System::new().block_on(async {
        let _ = HttpServer::new(|| App::new().route("/", actix_web::web::get().to(|| async { "ok" })))
            .bind("127.0.0.1:0")
            .unwrap()
            .run()
            .await;
    });
}
