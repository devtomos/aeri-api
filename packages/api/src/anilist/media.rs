use reqwest::{Client, Response};
use serde::Deserialize;
use serde_json::json;
use actix_web::{web, post, HttpResponse, Responder};
use colourful_logger::Logger;
use crate::anilist::queries::{get_query, QUERY_URL};
use lazy_static::lazy_static;
use crate::cache::redis::Redis;
use rand::Rng;
use crate::cache::proxy::{get_random_proxy, remove_proxy};
use crate::anilist::washed::{wash_media_data, wash_relation_data};

lazy_static! {
    static ref logger: Logger = Logger::default();
    static ref redis:  Redis  = Redis::new();
}

#[derive(Deserialize)]
struct RelationRequest {
    media_name: String,
    media_type: String,
}

#[derive(Deserialize)]
struct MediaRequest {
    media_id:   i32,
    media_type: String,
}

#[derive(Deserialize)]
struct RecommendRequest {
    media:      String,
    genres:     Option<Vec<String>>,
}

#[post("/relations")]
pub async fn relations_search(req: web::Json<RelationRequest>) -> impl Responder {
    if req.media_name.len() == 0 || req.media_type.len() == 0 {
        logger.error_single("No media name or type was included", "Relations");
        let bad_json = json!({"error": "No media name or type was included", "errorCode": 404});
        return HttpResponse::BadRequest().json(bad_json);
    }

    let get_proxy = get_random_proxy(&redis.get_client()).await.unwrap();
    let proxy = reqwest::Proxy::http(get_proxy.clone()).unwrap();
    let client = Client::builder().proxy(proxy).build().unwrap();
    let query:  String = get_query("relation_stats");
    let json:   serde_json::Value = json!({"query": query, "variables": {"search": req.media_name, "type": req.media_type.to_uppercase()}});
    logger.debug_single("Sending request with relational data", "Relations");

    let response: Response = client
            .post(QUERY_URL)
            .json(&json)
            .send()
            .await
            .unwrap();
    
    if response.status().as_u16() != 200 {
        if response.status().as_u16() == 403 {
            let _ = remove_proxy(&redis.get_client(), get_proxy.as_str()).await;
        }

        logger.error_single(format!("Request returned {} when trying to fetch data for {} with type {}", response.status().as_str(), req.media_name, req.media_type).as_str(), "Relations");
        let bad_json = json!({"error": "Request returned an error", "errorCode": response.status().as_u16()});
        return HttpResponse::BadRequest().json(bad_json);
    }
        
    let relations = response.json::<serde_json::Value>().await.unwrap();
    let relations = wash_relation_data(req.media_name.to_string(), relations).await;
    logger.debug_single("Returning relational data", "Relations");
    HttpResponse::Ok().json(relations)
}

#[post("/recommend")]
async fn recommend(req: web::Json<RecommendRequest>) -> impl Responder {
    let genres = req.genres.clone().unwrap_or(vec![]);
    let mut rng = rand::rng();

    let get_proxy = get_random_proxy(&redis.get_client()).await.unwrap();
    let proxy = reqwest::Proxy::http(get_proxy.clone()).unwrap();
    let client = Client::builder().proxy(proxy).build().unwrap();
    let recommendation_amount_query = get_query("recommendation_amount");
    let json = json!({
        "query": recommendation_amount_query,
        "variables": {
            "page": 1,
            "perPage": 50
        }
    });

    logger.debug_single(format!("Sending request to client with JSON query").as_str(), "Recommend");
    let response: Response = client
        .post(QUERY_URL)
        .json(&json)
        .send()
        .await
        .unwrap();
    
    if response.status().as_u16() != 200 {
        if response.status().as_u16() == 403 {
            let _ = remove_proxy(&redis.get_client(), get_proxy.as_str()).await;
        }

        let status = response.status();
        let response_text = response.text().await.unwrap();
        logger.error_single(format!("First request returned {} : {:?}", status, response_text).as_str(), "Recommend");
        let bad_json = json!({"error": "Request returned an error", "errorCode": status.as_u16(), "response": response_text});
        return HttpResponse::BadRequest().json(bad_json);
    }

    let recommend_amount = response.json::<serde_json::Value>().await.unwrap();
    let data = &recommend_amount["data"]["Page"];
    let last_page = data["pageInfo"]["lastPage"].as_i64().unwrap_or(1);
    logger.debug_single(&format!("Last Page set to : {}", last_page), "Recommend");

    let pages = rng.random_range(1..last_page);
    let mut recommend = get_recommendation(pages, genres.clone(), req.media.clone()).await;
    if recommend.get("error").is_some() {
        logger.debug_single("Bad JSON received, setting pages to 1 and retrying", "Recommend");
        recommend = get_recommendation(1, genres, req.media.clone()).await;
    }
    
    HttpResponse::Ok().json(recommend)
}

#[post("/media")]
pub async fn media_search(req: web::Json<MediaRequest>) -> impl Responder {
    if req.media_type.len() == 0 {
        logger.error_single("No type was included", "Media");
        let bad_json = json!({"error": "No type was included", "errorCode": 404});
        return HttpResponse::BadRequest().json(bad_json);
    }

    match redis.get(req.media_id.to_string()) {
        Ok(data) => {
            logger.debug_single("Found media data in cache. Returning cached data", "Media");
            let mut media_data: serde_json::Value = serde_json::from_str(data.as_str()).unwrap();
            media_data["dataFrom"] = "Cache".into();
            if let Some(_airing) = media_data["airing"].as_array().and_then(|arr| arr.get(0)) {
                media_data["airing"][0]["timeUntilAiring"] = redis.ttl(req.media_id.to_string()).unwrap().into();
            }
            media_data["leftUntilExpire"] = redis.ttl(req.media_id.to_string()).unwrap().into();
            return HttpResponse::Ok().json(media_data);
        },
        Err(_) => {
            logger.debug_single("No media data found in cache", "Media");
        }
    }

    let get_proxy = get_random_proxy(&redis.get_client()).await.unwrap();
    let proxy = reqwest::Proxy::http(get_proxy.clone()).unwrap();
    let client = Client::builder().proxy(proxy).build().unwrap();
    let query:  String = get_query("search");
    let json:   serde_json::Value = json!({"query": query, "variables": {"id": req.media_id, "type": req.media_type.to_uppercase()}});
    logger.debug_single("Sending request with relational data", "Media");

    let response: Response = client
            .post(QUERY_URL)
            .json(&json)
            .send()
            .await
            .unwrap();

    if response.status().as_u16() != 200 {
        if response.status().as_u16() == 403 {
            let _ = remove_proxy(&redis.get_client(), get_proxy.as_str()).await;
        }

        logger.error_single(format!("Request returned {} when trying to fetch data for {} with type {}", response.status().as_str(), req.media_id, req.media_type).as_str(), "Media");
        let bad_json = json!({"error": "Request returned an error", "errorCode": response.status().as_u16()});
        return HttpResponse::BadRequest().json(bad_json);
    }
        
    let media: serde_json::Value = response.json::<serde_json::Value>().await.unwrap();
    let media: serde_json::Value = wash_media_data(media).await;

    let _ = redis.set(media["id"].to_string(), media.clone().to_string());
    if media["airing"].as_array().unwrap().len() > 0 {
        logger.debug_single(&format!("{} is releasing, expiring cache when next episode is aired.", media["romaji"]), "Media");
        let _ = redis.expire(media["id"].to_string(), media["airing"][0]["timeUntilAiring"].as_i64().unwrap());
    } else {
        logger.debug_single(&format!("{} is not releasing, keep data for a week.", media["romaji"]), "Media");
        let _ = redis.expire(media["id"].to_string(), 86400);
    }

    HttpResponse::Ok().json(media)
}

async fn get_recommendation(pages: i64, genres: Vec<String>, media: String) -> serde_json::Value {
    let mut rng = rand::rng();
    let get_proxy = get_random_proxy(&redis.get_client()).await.unwrap();
    let proxy = reqwest::Proxy::http(get_proxy.clone()).unwrap();
    let client = Client::builder().proxy(proxy).build().unwrap();

    let recommendation_query = get_query("recommendation");
    let json = json!({
        "query": recommendation_query, 
        "variables": {
            "type": media, 
            "genres": genres, 
            "page": pages, 
            "perPage": 50
    }});

    logger.debug_single(format!("Sending request to client with JSON query:").as_str(), "Recommend");
    let response: Response = client
                .post(QUERY_URL)
                .json(&json)
                .send()
                .await
                .unwrap();
    
    if response.status().as_u16() != 200 {
        if response.status().as_u16() == 403 {
            let _ = remove_proxy(&redis.get_client(), get_proxy.as_str()).await;
        }

        let status = response.status();
        let response_text = response.text().await.unwrap();
        logger.error_single(format!("Second request returned {} : {:?}", status, response_text).as_str(), "Recommend");
        let bad_json = json!({"error": "Request returned an error", "errorCode": status.as_u16(), "response": response_text});
        return bad_json;
    }

    let recommeneded_ids = response.json::<serde_json::Value>().await.unwrap();
    let recommeneded_ids = &recommeneded_ids["data"]["Page"]["media"];

    logger.debug("Returning recommendations", "Recommend", false, recommeneded_ids.clone());

    let mut ids = Vec::new();
    for media in recommeneded_ids.as_array().unwrap() {
        if let Some(id) = media["id"].as_i64() {
            ids.push(id);
        }
    }

    if ids.len() == 0 {
        logger.error_single("No recommendations found", "Recommend");
        let bad_json = json!({"error": "No recommendations found"});
        return bad_json;
    }
    let random_choice = rng.random_range(0..ids.len());
    json!(ids[random_choice])
}