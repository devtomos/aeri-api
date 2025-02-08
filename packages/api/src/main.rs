use actix_web::{get, 
                web, 
                App, 
                HttpResponse, 
                HttpServer, 
                Responder};

use colourful_logger::Logger as Logger;
use lazy_static::lazy_static;
use std::env;
use std::thread;
use num_cpus;

mod anilist;
mod cache;
mod global;
mod client;
mod structs;

use anilist::media::{media_search, 
                    relations_search, 
                    recommend};

use anilist::user::{user_search, 
                    user_score, 
                    expire_user};

use anilist::search::{character_search, 
                    staff_search, 
                    studio_search};

use cache::redis::Redis;
use client::proxy::Proxy;

lazy_static! {
    static ref logger: Logger = Logger::default();
    static ref redis:  Redis  = Redis::new();
    static ref proxy:  Proxy  = Proxy::new();
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Welcome to the Ani API!")
}

async fn manual() -> impl Responder {
    HttpResponse::Ok().body("Ani API")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().unwrap_or_default();

    logger.info_single("Starting The API", "Main");
    let ip = env::var("API_HOST").unwrap_or("0.0.0.0".to_string());
    let port = env::var("API_PORT").unwrap().parse::<u16>().unwrap_or(8080);
    let check_proxy = env::var("API_PROXY").map_err(|_| {
        logger.error_single("API_PROXY environment variable not set", "Main");
    });

    if check_proxy.is_ok() {
        tokio::spawn(async move {
            let mut attempts: u8 = 0;
            while attempts < 10 {
                if let Err(e) = proxy.update_proxy_list().await {
                    logger.error_single(&format!("Failed to update proxy list (attempt {}): {:?}", attempts + 1, e), "Main");
                    thread::sleep(std::time::Duration::from_secs(10));
                    attempts += 1;
                }
            }
            if attempts == 10 {
                logger.error_single("Failed to update proxy list after 10 attempts", "Main");
            }
        });
    }

    logger.info_single(&format!("Listening on {}:{}", ip, port), "Main");    
    HttpServer::new(move || {
        App::new()
            .service(hello)
            .service(user_search)
            .service(user_score)
            .service(media_search)
            .service(relations_search)
            .service(expire_user)
            .service(recommend)
            .service(character_search)
            .service(staff_search)
            .service(studio_search)
            .route("/hey", web::get().to(manual))
    })
    .workers(num_cpus::get())
    .bind((ip, port))?
    .run()
    .await
}