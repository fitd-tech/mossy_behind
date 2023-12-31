#[macro_use] extern crate rocket;
use bson::Document;
use mongodb::results::{InsertOneResult, DeleteResult, UpdateResult, InsertManyResult};
use mongodb::{Client, options::ClientOptions};
use mongodb::error::Error;
use futures::stream::TryStreamExt;
use rocket::http::Status;
use rocket::request::{Request, Outcome, FromRequest};
use rocket::serde::{Serialize, Deserialize, json::Json};
use mongodb::bson;
use mongodb::options::FindOptions;
use reqwest;
use reqwest::Error as ReqwestError;
use base64::{Engine as _, engine::general_purpose};
use jsonwebtoken;
use jsonwebtoken::{DecodingKey, Validation, Algorithm};
use dotenv;

// https://www.mongodb.com/developer/languages/rust/serde-improvements/

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Book {
    _id: bson::oid::ObjectId,
    title: String,
    author: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Credentials {
    authorization_code: String,
    identity_token: String,
    nonce: String,
    user: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct AppleAuthKey {
    alg: String,
    e: String,
    kid: String,
    kty: String,
    n: String,
    #[serde(rename = "use")]
    use_alias: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct AppleAuthResponse {
    keys: Vec<AppleAuthKey>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct DecodedKeys {
    kid: String,
    decoded_e: Vec<u8>,
    decoded_n: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Claims {
    iss: String,
    aud: String,
    exp: i64,
    iat: i64,
    sub: String,
    c_hash: String,
    email: String,
    email_verified: String,
    auth_time: i64,
    nonce_supported: bool,
    nonce: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct User {
    _id: bson::oid::ObjectId,
    email: String,
    apple_user_id: String,
    token: String,
    is_admin: bool,
    should_color_scheme_use_system: bool,
    is_color_scheme_dark_mode: bool,
    color_theme: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct UserThemeData {
    apple_user_id: String,
    should_color_scheme_use_system: bool,
    is_color_scheme_dark_mode: bool,
    color_theme: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct UserData {
    apple_user_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Tag {
    _id: bson::oid::ObjectId,
    name: String,
    description: Option<String>,
    parent_tag: Option<bson::oid::ObjectId>,
    user: Option<bson::oid::ObjectId>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct NewTagData {
    name: String,
    description: Option<String>,
    parent_tag: Option<bson::oid::ObjectId>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct NewTaskData {
    name: String,
    frequency: i32,
    tags: Option<Vec<bson::oid::ObjectId>>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Task {
    _id: bson::oid::ObjectId,
    name: String,
    frequency: i32,
    tags: Option<Vec<bson::oid::ObjectId>>,
    user: Option<bson::oid::ObjectId>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct TaskWithLatestEvent {
    _id: bson::oid::ObjectId,
    name: String,
    frequency: i32,
    tags: Option<Vec<bson::oid::ObjectId>>,
    latest_event_date: Option<bson::DateTime>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Event {
    _id: bson::oid::ObjectId,
    task: bson::oid::ObjectId,
    date: bson::DateTime,
    user: Option<bson::oid::ObjectId>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct EventWithStringValues {
    _id: bson::oid::ObjectId,
    task: Option<String>,
    date: bson::DateTime,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct NewEventData {
    task: bson::oid::ObjectId,
    date: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct UpdateEventData {
    _id: bson::oid::ObjectId,
    date: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct DebugCreateTasksData {
    quantity: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct DebugCreateEventsData {
    quantity: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct DebugCreateTagsData {
    quantity: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct ReadParams {
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Debug)]
enum CredentialsError {
    FetchKeysError(ReqwestError),
    DotEnvError,
    NoClientIdError,
    NoDbAddressError,
    DeserializeJsonError,
    DecodeJwtError,
    NoKidError,
    NoMatchingKidError,
    InvalidKeySucceededError,
    MatchingKeyFailedError,
    DecodeComponentError,
    InvalidNonceError,
    DatabaseError,
}

#[get("/")]
async fn index() -> &'static str {
    return "Hello, world!";
}

// https://developer.apple.com/documentation/sign_in_with_apple/fetch_apple_s_public_key_for_verifying_token_signature
// https://stackoverflow.com/questions/66067321/marshal-appleids-public-key-to-rsa-publickey
// https://developer.apple.com/documentation/sign_in_with_apple/sign_in_with_apple_rest_api/verifying_a_user
// https://jwt.io/ to decode JWT
async fn validate_credentials(credentials: Credentials) -> Result<User, CredentialsError> {
    let keys_response = match reqwest::get("https://appleid.apple.com/auth/keys").await {
        Ok(_keys_response) => _keys_response,
        Err(_keys_response) => return Err(CredentialsError::FetchKeysError(_keys_response))
    };
    let deserialized_keys_response = match keys_response.json::<AppleAuthResponse>().await {
        Ok(_deserialized_keys_response) => _deserialized_keys_response,
        Err(_deserialized_keys_response) => return Err(CredentialsError::DeserializeJsonError)
    };

    let credential_header = match jsonwebtoken::decode_header(&credentials.identity_token) {
        Ok(_credential_header) => _credential_header,
        Err(_credential_header) => return Err(CredentialsError::DecodeJwtError)
    };
    let Some(credential_kid) = credential_header.kid else {
        return Err(CredentialsError::NoKidError)
    };

    let _dotenv_result = match dotenv::dotenv() {
        Ok(_dotenv_result) => _dotenv_result,
        Err(_) => return Err(CredentialsError::DotEnvError),
    };
    let apple_client_id = match dotenv::var("APPLE_CLIENT_ID") {
        Ok(_apple_client_id) => _apple_client_id,
        Err(_) => return Err(CredentialsError::NoClientIdError)
    };

    // We can specify validation predicates here per this list:
    // https://developer.apple.com/documentation/sign_in_with_apple/sign_in_with_apple_rest_api/verifying_a_user
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&["https://appleid.apple.com"]);
    validation.set_audience(&[apple_client_id]);

    let mut keys_iterator = deserialized_keys_response.keys.into_iter();
    let Some(matching_key) = keys_iterator.find(|key| key.kid == credential_kid) else {
        return Err(CredentialsError::NoMatchingKidError)
    };

    // Make sure an invalid key fails, if one exists in the response
    if let Some(invalid_key) = keys_iterator.find(|key| key.kid != credential_kid) {
        let decoded_n = match general_purpose::URL_SAFE_NO_PAD.decode(invalid_key.n) {
            Ok(_decoded_n) => _decoded_n,
            Err(_decoded_n) => return Err(CredentialsError::DecodeComponentError)
        };
        let decoded_e = match general_purpose::URL_SAFE_NO_PAD.decode(invalid_key.e) {
            Ok(_decoded_e) => _decoded_e,
            Err(_decoded_e) => return Err(CredentialsError::DecodeComponentError)
        };
        let decoding_key = DecodingKey::from_rsa_raw_components(&decoded_n, &decoded_e);
        let _claims = match jsonwebtoken::decode::<Claims>(&credentials.identity_token, &decoding_key, &validation) {
            Ok(_) => return Err(CredentialsError::InvalidKeySucceededError),
            Err(_) => println!("Invalid key failed as expected! 👍"),
        };
    };

    let decoded_n = match general_purpose::URL_SAFE_NO_PAD.decode(matching_key.n) {
        Ok(_decoded_n) => _decoded_n,
        Err(_decoded_n) => return Err(CredentialsError::DecodeComponentError)
    };
    let decoded_e = match general_purpose::URL_SAFE_NO_PAD.decode(matching_key.e) {
        Ok(_decoded_e) => _decoded_e,
        Err(_decoded_e) => return Err(CredentialsError::DecodeComponentError)
    };
    let decoding_key = DecodingKey::from_rsa_raw_components(&decoded_n, &decoded_e);
    let claims = match jsonwebtoken::decode::<Claims>(&credentials.identity_token, &decoding_key, &validation) {
        Ok(_claims) => _claims,
        Err(_claims) => return Err(CredentialsError::MatchingKeyFailedError),
    };

    let Some(claims_nonce) = &claims.claims.nonce else {
        return Err(CredentialsError::InvalidNonceError)
    };
    if *claims_nonce != credentials.nonce {
        return Err(CredentialsError::InvalidNonceError)
    }

    let mongodb_address = match dotenv::var("MONGODB_ADDRESS") {
        Ok(_mongodb_address) => _mongodb_address,
        Err(_) => return Err(CredentialsError::NoDbAddressError)
    };
    
    let mut client_options = match ClientOptions::parse(mongodb_address).await {
        Ok(_client_options) => _client_options,
        Err(_) => return Err(CredentialsError::DatabaseError)
    };
    client_options.app_name = Some("mossy".to_string());
    let client = match Client::with_options(client_options) {
        Ok(_client) => _client,
        Err(_) => return Err(CredentialsError::DatabaseError)
    };
    let db = client.database("mossy");

    let users = db.collection::<User>("users");

    let filter = bson::doc! {
        "apple_user_id": credentials.user.clone(),
    };
    let existing_user_option = match users.find_one(filter, None).await {
        Ok(_existing_user) => _existing_user,
        Err(_existing_user) => return Err(CredentialsError::DatabaseError)
    };
    let token = bson::uuid::Uuid::new().to_string();
    let token_create_user_copy = token.clone();
    let token_update_user_copy = token.clone();
    let saved_user = if existing_user_option.is_none() {
        let email_copy = claims.claims.email.clone();
        let user_copy = credentials.user.clone();
        let user = User {
            _id: bson::oid::ObjectId::new(),
            email: email_copy,
            apple_user_id: user_copy,
            token: token_create_user_copy,
            should_color_scheme_use_system: false,
            is_color_scheme_dark_mode: false,
            color_theme: 1,
            is_admin: false,
        };
        let user_copy = user.clone();
        match users.insert_one(user_copy, None).await {
            Ok(_user_result) => _user_result,
            Err(_) => return Err(CredentialsError::DatabaseError)
        };
        user
    } else {
        let updated_user = bson::doc! {
            "$set": {
                "token": token,
            }
        };
        let mut existing_user = existing_user_option.unwrap();
        let filter = bson::doc!{"_id": existing_user._id };
        match users.update_one(filter, updated_user, None).await {
            Ok(_user_result) => _user_result,
            Err(_) => return Err(CredentialsError::DatabaseError)
        };
        existing_user.token = token_update_user_copy;
        existing_user
    };

    Ok(saved_user)
}

async fn read_user_action(token: Token<'_>, user_data: UserData) -> Result<User, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let filter = bson::doc! {
        "apple_user_id": user_data.apple_user_id.clone(),
        "token": token_value,
    };

    let user_option = match users.find_one(filter, None).await {
        Ok(_user) => _user,
        Err(_) => todo!()
    };

    if let Some(user) = user_option {
        Ok(user)
    } else {
        todo!()
    }
}

async fn update_user_theme_action(token: Token<'_>, user_theme_data: UserThemeData) -> Result<UpdateResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
        "apple_user_id": user_theme_data.apple_user_id,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    let updated_user = bson::doc! {
        "$set": {
            "should_color_scheme_use_system": user_theme_data.should_color_scheme_use_system,
            "is_color_scheme_dark_mode": user_theme_data.is_color_scheme_dark_mode,
            "color_theme": user_theme_data.color_theme,
        }
    };

    let filter = bson::doc!{"_id": user._id };

    let user_result = users.update_one(filter, updated_user, None).await;

    match user_result {
        Ok(_user_result) => Ok(_user_result),
        Err(_) => todo!(),
    }
}

async fn read_tasks_action(token: Token<'_>, params: ReadParams) -> Result<Vec<Document>, Error> {
    let limit = params.limit.unwrap_or(0);
    let offset = params.offset.unwrap_or(0);

    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tasks = db.collection::<Task>("tasks");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    let tasks_filter = vec! [
        bson::doc! {
            "$match": {
                "user": user._id,
            }
        },
        bson::doc! {
            "$lookup": {
                "from": "events",
                "localField": "_id",
                "foreignField": "task",
                "as": "event_mapping", 
            },
        },
        bson::doc! {
            "$set": {
            "event_mapping": {
                "$sortArray": {
                "input": "$event_mapping",
                "sortBy": {
                    "date": -1
                }
                }
            }
            }
        },
        bson::doc! {
            "$set": {
            "event_mapping": {
                "$first": "$event_mapping"
            }
            }
        },
        bson::doc! {
            "$set": {
            "latest_event_date": "$event_mapping.date",
            "time_since_latest_event": {
                "$dateDiff": {
                "startDate": "$event_mapping.date",
                "endDate": bson::DateTime::now(),
                "unit": "millisecond",
                }
            }
            }
        },
        bson::doc! {
            "$set": {
            "frequency_in_milliseconds_raw": {
                "$multiply": [
                1000,
                60,
                60,
                24,
                "$frequency"
                ]
            }
            }
        },
        bson::doc! {
            "$set": {
            "frequency_in_milliseconds": {
                "$toLong": "$frequency_in_milliseconds_raw"
            }
            }
        },
        bson::doc! {
            "$set": {
            "moss": {
                "$subtract": [
                "$time_since_latest_event",
                "$frequency_in_milliseconds"
                ]
            }
            }
        },
        bson::doc! {
            "$unset": [
                "event_mapping",
                "frequency_in_milliseconds_raw",
                "frequency_in_milliseconds",
            ]
        },
        bson::doc! {
            // We also need to sort by a unique value (_id) to ensure we don't get duplicates in pagination
            // We should eventually create a compound index for this: https://www.mongodb.com/docs/manual/tutorial/sort-results-with-indexes/#sort-on-multiple-fields
            "$sort": {
                "moss": -1,
                "_id": -1,
            }
        },
        bson::doc! {
            "$skip": offset
        },
        bson::doc! {
            "$limit": limit
        },
    ];
    let mut tasks_cursor = tasks.aggregate(tasks_filter, None).await?;

    let mut tasks_list = Vec::new();

    while let Some(task) = tasks_cursor.try_next().await? {
        tasks_list.push(task);
    }

    Ok(tasks_list)
}

async fn read_events_action(token: Token<'_>, params: ReadParams) ->Result<Vec<Event>, Error> {
    let limit = params.limit.unwrap_or(0);
    let offset = params.offset.unwrap_or(0);

    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let events = db.collection::<Event>("events");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    let events_filter = bson::doc! {
        "user": user._id,
    };
    let sort_option = bson::doc! {
        "date": -1,
        "_id": -1,
    };
    let options = FindOptions::builder().sort(sort_option).skip(Some(u64::from(offset))).limit(Some(i64::from(limit))).build();
    let mut cursor = events.find(events_filter, options).await?;

    let mut events_list = Vec::new();

    while let Some(event) = cursor.try_next().await? {
        events_list.push(event);
    }

    Ok(events_list)
}

async fn read_events_string_action(token: Token<'_>, params: ReadParams) ->Result<Vec<EventWithStringValues>, Error> {
    let limit = params.limit.unwrap_or(0);
    let offset = params.offset.unwrap_or(0);

    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let events = db.collection::<Event>("events");
    let tasks = db.collection::<Task>("tasks");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    let events_filter = bson::doc! {
        "user": user._id,
    };
    let sort_option = bson::doc! {
        "date": -1,
        "_id": -1,
    };
    let options = FindOptions::builder().sort(sort_option).skip(Some(u64::from(offset))).limit(Some(i64::from(limit))).build();
    let mut cursor = events.find(events_filter, options).await?;

    let mut events_list = Vec::new();

    while let Some(event) = cursor.try_next().await? {
        let filter = bson::doc! {
            "_id": event.task,
        };
        let task = tasks.find_one(filter, None).await?;

        let event_with_string_values = match task {
            Some(_task) => EventWithStringValues {
                _id: event._id,
                task: Some(_task.name),
                date: event.date,
            },
            None => EventWithStringValues {
                _id: event._id,
                task: None,
                date: event.date,
            }
        };
        events_list.push(event_with_string_values);
    }

    Ok(events_list)
}

async fn read_tags_action(token: Token<'_>, params: ReadParams) ->Result<Vec<Tag>, Error> {
    let limit = params.limit.unwrap_or(0);
    let offset = params.offset.unwrap_or(0);

    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tags = db.collection::<Tag>("tags");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    let tags_filter = bson::doc! {
        "user": user._id,
    };
    let sort_option = bson::doc! {
        "name": 1,
        "_id": -1,
    };
    let options = FindOptions::builder().sort(sort_option).skip(Some(u64::from(offset))).limit(Some(i64::from(limit))).build();
    let mut cursor = tags.find(tags_filter, options).await?;

    let mut tags_list = Vec::new();

    while let Some(tag) = cursor.try_next().await? {
        tags_list.push(tag);
    }

    Ok(tags_list)
}

async fn create_task_action(token: Token<'_>, task_data: NewTaskData) -> Result<InsertOneResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tasks = db.collection::<Task>("tasks");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    let new_task = Task {
        _id: bson::oid::ObjectId::new(),
        name: task_data.name,
        frequency: task_data.frequency,
        tags: task_data.tags,
        user: Some(user._id),
    };

    let task_result = tasks.insert_one(new_task, None).await;

    match task_result {
        Ok(_task_result) => Ok(_task_result),
        Err(_) => todo!(),
    }
}

async fn create_event_action(token: Token<'_>, event_data: NewEventData) -> Result<InsertOneResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let events = db.collection::<Event>("events");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    let date = match bson::DateTime::parse_rfc3339_str(event_data.date) {
        Ok(_date) => _date,
        Err(_date) => todo!()
    };
    let new_event = Event {
        _id: bson::oid::ObjectId::new(),
        task: event_data.task,
        date: date,
        user: Some(user._id),
    };

    let event_result = events.insert_one(new_event, None).await;

    match event_result {
        Ok(_event_result) => Ok(_event_result),
        Err(_) => todo!(),
    }
}

async fn create_tag_action(token: Token<'_>, tag_data: NewTagData) -> Result<InsertOneResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tags = db.collection::<Tag>("tags");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    let new_tag = Tag {
        _id: bson::oid::ObjectId::new(),
        name: tag_data.name,
        description: tag_data.description,
        parent_tag: tag_data.parent_tag,
        user: Some(user._id),
    };

    let tag_result = tags.insert_one(new_tag, None).await;

    match tag_result {
        Ok(_tag_result) => Ok(_tag_result),
        Err(_) => todo!(),
    }
}

async fn update_task_action(token: Token<'_>, task_data: Task) -> Result<UpdateResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tasks = db.collection::<Task>("tasks");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    // Make sure the task to update belongs to the user
    let task_filter = bson::doc! {
        "_id": task_data._id,
    };
    let Some(task) = tasks.find_one(task_filter, None).await? else {
        todo!()
    };
    if task.user != Some(user._id) {
        todo!()
    };

    let updated_task = bson::doc! {
        "$set": {
            "name": task_data.name,
            "frequency": task_data.frequency,
            "tags": task_data.tags,
        }
    };

    let filter = bson::doc!{"_id": task_data._id };

    let task_result = tasks.update_one(filter, updated_task, None).await;

    match task_result {
        Ok(_task_result) => Ok(_task_result),
        Err(_) => todo!(),
    }
}

async fn update_event_action(token: Token<'_>, event_data: UpdateEventData) -> Result<UpdateResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let events = db.collection::<Event>("events");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    // Make sure the event to update belongs to the user
    let event_filter = bson::doc! {
        "_id": event_data._id,
    };
    let Some(event) = events.find_one(event_filter, None).await? else {
        todo!()
    };
    if event.user != Some(user._id) {
        todo!()
    };

    let date = match bson::DateTime::parse_rfc3339_str(event_data.date) {
        Ok(_date) => _date,
        Err(_date) => todo!()
    };
    let updated_event = bson::doc! {
        "$set": {
            "date": date,
        }
    };

    let filter = bson::doc!{"_id": event_data._id };

    let event_result = events.update_one(filter, updated_event, None).await;

    match event_result {
        Ok(_event_result) => Ok(_event_result),
        Err(_) => todo!(),
    }
}

async fn update_tag_action(token: Token<'_>, tag_data: Tag) -> Result<UpdateResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tags = db.collection::<Tag>("tags");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    // Make sure the tag to update belongs to the user
    let tag_filter = bson::doc! {
        "_id": tag_data._id,
    };
    let Some(tag) = tags.find_one(tag_filter, None).await? else {
        todo!()
    };
    if tag.user != Some(user._id) {
        todo!()
    };

    let updated_tag = bson::doc! {
        "$set": {
            "name": tag_data.name,
            "description": tag_data.description,
            "parent_tag": tag_data.parent_tag,
        }
    };

    let filter = bson::doc!{"_id": tag_data._id };

    let tag_result = tags.update_one(filter, updated_tag, None).await;

    match tag_result {
        Ok(_tag_result) => Ok(_tag_result),
        Err(_) => todo!(),
    }
}

async fn delete_tasks_action(token: Token<'_>, tasks_data: Vec<bson::oid::ObjectId>) -> Result<DeleteResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tasks = db.collection::<Task>("tasks");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    // Make sure the task to delete belongs to the user
    let tasks_data_copy = tasks_data.clone();
    let task_filter = bson::doc! {
        "_id": {
            "$in": tasks_data_copy,
        },
    };
    let mut tasks_cursor = tasks.find(task_filter, None).await?;
    while let Some(task) = tasks_cursor.try_next().await? {
        if task.user != Some(user._id) {
            todo!()
        };
    }

    let filter = bson::doc!{"_id": { "$in": tasks_data }};

    let tasks_result = tasks.delete_many(filter, None).await;

    match tasks_result {
        Ok(_tasks_result) => Ok(_tasks_result),
        Err(_) => todo!(),
    }
}

async fn delete_events_action(token: Token<'_>, events_data: Vec<bson::oid::ObjectId>) -> Result<DeleteResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let events = db.collection::<Event>("events");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    // Make sure the event to delete belongs to the user
    let events_data_copy = events_data.clone();
    let event_filter = bson::doc! {
        "_id": {
            "$in": events_data_copy,
        },
    };
    let mut events_cursor = events.find(event_filter, None).await?;
    while let Some(event) = events_cursor.try_next().await? {
        if event.user != Some(user._id) {
            todo!()
        };
    }

    let filter = bson::doc!{"_id": { "$in": events_data }};

    let events_result = events.delete_many(filter, None).await;

    match events_result {
        Ok(_events_result) => Ok(_events_result),
        Err(_) => todo!(),
    }
}

async fn delete_tags_action(token: Token<'_>, tags_data: Vec<bson::oid::ObjectId>) -> Result<DeleteResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tags = db.collection::<Tag>("tags");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };

    // Make sure the tag to delete belongs to the user
    let tags_data_copy = tags_data.clone();
    let tag_filter = bson::doc! {
        "_id": {
            "$in": tags_data_copy,
        },
    };
    let mut tags_cursor = tags.find(tag_filter, None).await?;
    while let Some(tag) = tags_cursor.try_next().await? {
        if tag.user != Some(user._id) {
            todo!()
        };
    }

    let filter = bson::doc!{"_id": { "$in": tags_data }};

    let tags_result = tags.delete_many(filter, None).await;

    match tags_result {
        Ok(_tags_result) => Ok(_tags_result),
        Err(_) => todo!(),
    }
}

async fn debug_create_tasks_action(token: Token<'_>, data: DebugCreateTasksData) -> Result<InsertManyResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tasks = db.collection::<Task>("tasks");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };
    if user.is_admin == false {
        todo!()
    };

    let quantity_to_create = data.quantity;

    let mut iteration = 0;
    let mut new_tasks = Vec::new();

    while iteration < quantity_to_create {
        let new_task = Task {
            _id: bson::oid::ObjectId::new(),
            name: String::from(iteration.to_string()),
            frequency: 7,
            tags: None,
            user: Some(user._id),
        };
        new_tasks.push(new_task);
        iteration += 1;
    }


    let task_result = tasks.insert_many(new_tasks, None).await;

    match task_result {
        Ok(_task_result) => Ok(_task_result),
        Err(_) => todo!(),
    }
}

async fn debug_delete_tasks_action(token: Token<'_>) -> Result<DeleteResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tasks = db.collection::<Task>("tasks");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };
    if user.is_admin == false {
        todo!()
    };

    let filter = bson::doc!{"user": user._id};

    let tasks_result = tasks.delete_many(filter, None).await;

    match tasks_result {
        Ok(_tasks_result) => Ok(_tasks_result),
        Err(_) => todo!(),
    }
}

async fn debug_create_events_action(token: Token<'_>) -> Result<InsertManyResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tasks = db.collection::<Task>("tasks");
    let events = db.collection::<Event>("events");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };
    if user.is_admin == false {
        todo!()
    };

    let mut new_events = Vec::new();

    let tasks_filter = bson::doc! {
        "user": user._id,
    };
    let mut tasks_cursor = tasks.find(tasks_filter, None).await?;
    while let Some(task) = tasks_cursor.try_next().await? {
        let date = match bson::DateTime::parse_rfc3339_str("2023-10-01T05:43:48.487Z") {
            Ok(_date) => _date,
            Err(_date) => todo!(),
        };
        let new_event = Event {
            _id: bson::oid::ObjectId::new(),
            task: task._id,
            date: date,
            user: Some(user._id),
        };
        new_events.push(new_event);
    }

    let events_result = events.insert_many(new_events, None).await;

    match events_result {
        Ok(_events_result) => Ok(_events_result),
        Err(_) => todo!(),
    }
}

async fn debug_delete_events_action(token: Token<'_>) -> Result<DeleteResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let events = db.collection::<Event>("events");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };
    if user.is_admin == false {
        todo!()
    };

    let filter = bson::doc!{"user": user._id};

    let events_result = events.delete_many(filter, None).await;

    match events_result {
        Ok(_events_result) => Ok(_events_result),
        Err(_) => todo!(),
    }
}

async fn debug_create_tags_action(token: Token<'_>, data: DebugCreateTagsData) -> Result<InsertManyResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tags = db.collection::<Tag>("tags");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };
    if user.is_admin == false {
        todo!()
    };

    let quantity_to_create = data.quantity;

    let mut iteration = 0;
    let mut new_tags = Vec::new();

    while iteration < quantity_to_create {
        let new_tag = Tag {
            _id: bson::oid::ObjectId::new(),
            name: String::from(iteration.to_string()),
            description: None,
            parent_tag: None,
            user: Some(user._id),
        };
        new_tags.push(new_tag);
        iteration += 1;
    }


    let tags_result = tags.insert_many(new_tags, None).await;

    match tags_result {
        Ok(_tags_result) => Ok(_tags_result),
        Err(_) => todo!(),
    }
}

async fn debug_delete_tags_action(token: Token<'_>) -> Result<DeleteResult, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;
    let db = client.database("mossy");

    let users = db.collection::<User>("users");
    let tags = db.collection::<Task>("tags");

    let mut token_split = token.clone().0.split(" ");
    let Some(token_value) = token_split.nth(1) else {
        todo!()
    };

    let user_filter = bson::doc! {
        "token": token_value,
    };
    let Some(user) = users.find_one(user_filter, None).await? else {
        todo!()
    };
    if user.is_admin == false {
        todo!()
    };

    let filter = bson::doc!{"user": user._id};

    let tags_result = tags.delete_many(filter, None).await;

    match tags_result {
        Ok(_tags_result) => Ok(_tags_result),
        Err(_) => todo!(),
    }
}

#[derive(Debug, Clone)]
struct Token<'r>(&'r str);

#[derive(Debug)]
enum TokenError {
    Missing,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Token<'r> {
    type Error = TokenError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(token) = request.headers().get_one("Authorization") {
            return Outcome::Success(Token(token))
        } else {
            return Outcome::Failure((Status::BadRequest, TokenError::Missing))
        };
    }
}

#[catch(500)]
fn internal_error() -> &'static str {
    "The server encountered an internal error."
}

#[post("/api/log-in", format="json", data="<credentials>")]
async fn log_in(credentials: Json<Credentials>) -> Result<Json<User>, Status> {
    let deserialized_credentials = credentials.into_inner();
    let log_in_result = validate_credentials(deserialized_credentials).await;

    match log_in_result {
        Ok(_log_in_result) => Ok(Json(_log_in_result)),
        Err(error) => {
            // Print the specific error for dubugging until we can log it properly
            println!("{:?}", error);
            return Err(Status::InternalServerError)
        },
    }
}

#[post("/api/user", format="json", data="<user>")]
async fn read_user(token: Token<'_>, user: Json<UserData>) -> Result<Json<User>, Status> {
    let deserialized_user = user.into_inner();
    let user = read_user_action(token, deserialized_user).await;

    match user {
        Ok(user_result) => Ok(Json(user_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[patch("/api/user/theme", format="json", data="<theme_data>")]
async fn update_user_theme(token: Token<'_>, theme_data: Json<UserThemeData>) -> Result<Json<UpdateResult>, Status> {
    let deserialized_theme = theme_data.into_inner();
    let theme_result = update_user_theme_action(token, deserialized_theme).await;

    match theme_result {
        Ok(_theme) => Ok(Json(_theme)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/api/tasks?<limit>&<offset>", format="json")]
async fn read_tasks(token: Token<'_>, limit: Option<u32>, offset: Option<u32>) -> Result<Json<Vec<Document>>, Status> {
    let params = ReadParams {
        limit: limit,
        offset: offset,
    };
    let tasks = read_tasks_action(token, params).await;

    match tasks {
        Ok(tasks_result) => Ok(Json(tasks_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/tasks", format="json", data="<task>")]
async fn create_task(token: Token<'_>, task: Json<NewTaskData>) -> Result<Json<InsertOneResult>, Status> {
    let deserialized_task = task.into_inner();
    let task = create_task_action(token, deserialized_task).await;

    match task {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[patch("/api/tasks", format="json", data="<task>")]
async fn update_task(token: Token<'_>, task: Json<Task>) -> Result<Json<UpdateResult>, Status> {
    let deserialized_task = task.into_inner();
    let task = update_task_action(token, deserialized_task).await;

    match task {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/tasks", format="json", data="<tasks>")]
async fn delete_tasks(token: Token<'_>, tasks: Json<Vec<bson::oid::ObjectId>>) -> Result<Json<DeleteResult>, Status> {
    let deserialized_tasks_list = tasks.into_inner();
    let tasks = delete_tasks_action(token, deserialized_tasks_list).await;

    match tasks {
        Ok(tasks_result) => Ok(Json(tasks_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/api/events?<limit>&<offset>", format="json")]
async fn read_events(token: Token<'_>, limit: Option<u32>, offset: Option<u32>) -> Result<Json<Vec<Event>>, Status> {
    let params = ReadParams {
        limit: limit,
        offset: offset,
    };
    let events = read_events_action(token, params).await;

    match events {
        Ok(events_result) => Ok(Json(events_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/api/events-string?<limit>&<offset>", format="json")]
async fn read_events_string(token: Token<'_>, limit: Option<u32>, offset: Option<u32>) -> Result<Json<Vec<EventWithStringValues>>, Status> {
    let params = ReadParams {
        limit: limit,
        offset: offset,
    };
    let events = read_events_string_action(token, params).await;

    match events {
        Ok(events_result) => Ok(Json(events_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/events", format="json", data="<event>")]
async fn create_event(token: Token<'_>, event: Json<NewEventData>) -> Result<Json<InsertOneResult>, Status> {
    let deserialized_event = event.into_inner();
    let event = create_event_action(token, deserialized_event).await;

    match event {
        Ok(event_result) => Ok(Json(event_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[patch("/api/events", format="json", data="<event>")]
async fn update_event(token: Token<'_>, event: Json<UpdateEventData>) -> Result<Json<UpdateResult>, Status> {
    let deserialized_event = event.into_inner();
    let event = update_event_action(token, deserialized_event).await;

    match event {
        Ok(event_result) => Ok(Json(event_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/events", format="json", data="<events>")]
async fn delete_events(token: Token<'_>, events: Json<Vec<bson::oid::ObjectId>>) -> Result<Json<DeleteResult>, Status> {
    let deserialized_events_list = events.into_inner();
    let events = delete_events_action(token, deserialized_events_list).await;

    match events {
        Ok(events_result) => Ok(Json(events_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/api/tags?<limit>&<offset>", format="json")]
async fn read_tags(token: Token<'_>, limit: Option<u32>, offset: Option<u32>) -> Result<Json<Vec<Tag>>, Status> {
    let params = ReadParams {
        limit: limit,
        offset: offset,
    };
    let tags = read_tags_action(token, params).await;

    match tags {
        Ok(tags_result) => Ok(Json(tags_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/tags", format="json", data="<tag>")]
async fn create_tag(token: Token<'_>, tag: Json<NewTagData>) -> Result<Json<InsertOneResult>, Status> {
    let deserialized_tag = tag.into_inner();
    let tag = create_tag_action(token, deserialized_tag).await;

    match tag {
        Ok(tag_result) => Ok(Json(tag_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[patch("/api/tags", format="json", data="<tag>")]
async fn update_tag(token: Token<'_>, tag: Json<Tag>) -> Result<Json<UpdateResult>, Status> {
    let deserialized_tag = tag.into_inner();
    let tag = update_tag_action(token, deserialized_tag).await;

    match tag {
        Ok(tag_result) => Ok(Json(tag_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/tags", format="json", data="<tags>")]
async fn delete_tags(token: Token<'_>, tags: Json<Vec<bson::oid::ObjectId>>) -> Result<Json<DeleteResult>, Status> {
    let deserialized_tags_list = tags.into_inner();
    let tags = delete_tags_action(token, deserialized_tags_list).await;

    match tags {
        Ok(tags_result) => Ok(Json(tags_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/debug/tasks", format="json", data="<data>")]
async fn debug_create_tasks(token: Token<'_>, data: Json<DebugCreateTasksData>) -> Result<Json<InsertManyResult>, Status> {
    let deserialized_data = data.into_inner();
    let tasks = debug_create_tasks_action(token, deserialized_data).await;

    match tasks {
        Ok(tasks_result) => Ok(Json(tasks_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/debug/tasks", format="json")]
async fn debug_delete_tasks(token: Token<'_>) -> Result<Json<DeleteResult>, Status> {
    let delete_result = debug_delete_tasks_action(token).await;

    match delete_result {
        Ok(_delete_result) => Ok(Json(_delete_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/debug/events", format="json")]
async fn debug_create_events(token: Token<'_>) -> Result<Json<InsertManyResult>, Status> {
    let events = debug_create_events_action(token).await;

    match events {
        Ok(events_result) => Ok(Json(events_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/debug/events", format="json")]
async fn debug_delete_events(token: Token<'_>) -> Result<Json<DeleteResult>, Status> {
    let delete_result = debug_delete_events_action(token).await;

    match delete_result {
        Ok(_delete_result) => Ok(Json(_delete_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/debug/tags", format="json", data="<data>")]
async fn debug_create_tags(token: Token<'_>, data: Json<DebugCreateTagsData>) -> Result<Json<InsertManyResult>, Status> {
    let deserialized_data = data.into_inner();
    let tags = debug_create_tags_action(token, deserialized_data).await;

    match tags {
        Ok(tags_result) => Ok(Json(tags_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/debug/tags", format="json")]
async fn debug_delete_tags(token: Token<'_>) -> Result<Json<DeleteResult>, Status> {
    let delete_result = debug_delete_tags_action(token).await;

    match delete_result {
        Ok(_delete_result) => Ok(Json(_delete_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .register("/", catchers![internal_error])
        .mount("/", routes![index])
        .mount("/", routes![log_in])
        .mount("/", routes![read_user])
        .mount("/", routes![update_user_theme])
        .mount("/", routes![read_tasks])
        .mount("/", routes![create_task])
        .mount("/", routes![update_task])
        .mount("/", routes![delete_tasks])
        .mount("/", routes![read_events])
        .mount("/", routes![read_events_string])
        .mount("/", routes![create_event])
        .mount("/", routes![update_event])
        .mount("/", routes![delete_events])
        .mount("/", routes![read_tags])
        .mount("/", routes![create_tag])
        .mount("/", routes![update_tag])
        .mount("/", routes![delete_tags])
        .mount("/", routes![debug_create_tasks])
        .mount("/", routes![debug_delete_tasks])
        .mount("/", routes![debug_create_events])
        .mount("/", routes![debug_delete_events])
        .mount("/", routes![debug_create_tags])
        .mount("/", routes![debug_delete_tags])
}