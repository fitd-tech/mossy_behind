#[macro_use] extern crate rocket;
use mongodb::{Client, options::ClientOptions};
// use mongodb::bson::extjson::de::Error;
// use mongodb::bson::{doc, Document};
use mongodb::Collection;
use mongodb::error::Error;
use futures::stream::TryStreamExt;
use rocket::http::Status;
use rocket::serde::{Serialize, Deserialize, json::Json};

// https://www.mongodb.com/developer/languages/rust/serde-improvements/

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Book {
    title: String,
    author: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Task {
    name: String,
    frequency: i32,
}

// #[tokio::main]
async fn fetch_books() -> Result<Option<Book>, Error> {
    println!("Hello, world!");

    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;

    client_options.app_name = Some("mossy".to_string());

    let client = Client::with_options(client_options)?;

    for db_name in client.list_database_names(None, None).await? {
        println!("db_name: {}", db_name);
    }

    let db = client.database("mossy");

    for collection_name in db.list_collection_names(None).await? {
        println!("collection_name: {}", collection_name);
    }

    let books = db.collection::<Book>("books");

    /* let docs = vec![    
        Book { title: "1984".to_string(), author: "George Orwell".to_string() },
        Book { title: "Animal Farm".to_string(), author: "George Orwell".to_string() },
        Book { title: "The Great Gatsby".to_string(), author: "F. Scott Fitzgerald".to_string() },
    ]; */

    // Disable insert since we have a LOT of books by now
    // We should figure out how to get a count of all records
    // books.insert_many(docs, None).await?;

    let mut cursor = books.find(None, None).await?;

    while let Some(book) = cursor.try_next().await? {
        println!("book: {:?}", book);
    }

    let book = books.find_one(None, None).await;

    match book {
        Ok(book_result) => Ok(book_result),
        Err(_) => todo!(),
    }
}

#[get("/")]
async fn index() -> &'static str {
    return "Hello, world!";
}

#[get("/api/books", format="json")]
async fn get_books_list() -> Result<Json<Option<Book>>, Status> {
    let book = fetch_books().await;
    println!("Found book:");
    println!("book {:?}", book);

    /* let book: Result<Option<Book>, Error> = books?.find_one(None, None).await;

    match books {
        Ok(book_result) => Ok(Json(book_result)),
        Err(_) => Err(Status::InternalServerError),
    } */

    match book {
        Ok(book_result) => Ok(Json(book_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

async fn fetch_tasks() -> Result<Option<Task>, Error> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("mossy".to_string());
    let client = Client::with_options(client_options)?;

    for db_name in client.list_database_names(None, None).await? {
        println!("db_name: {}", db_name);
    }

    let db = client.database("mossy");

    for collection_name in db.list_collection_names(None).await? {
        println!("collection_name: {}", collection_name);
    }

    let tasks = db.collection::<Task>("tasks");

    /* let docs = vec![    
        Book { title: "1984".to_string(), author: "George Orwell".to_string() },
        Book { title: "Animal Farm".to_string(), author: "George Orwell".to_string() },
        Book { title: "The Great Gatsby".to_string(), author: "F. Scott Fitzgerald".to_string() },
    ]; */

    // Disable insert since we have a LOT of books by now
    // We should figure out how to get a count of all records
    // books.insert_many(docs, None).await?;

    let mut cursor = tasks.find(None, None).await?;

    while let Some(task) = cursor.try_next().await? {
        println!("task: {:?}", task);
    }

    let task = tasks.find_one(None, None).await;

    match task {
        Ok(task_result) => Ok(task_result),
        Err(_) => todo!(),
    }
}

#[get("/api/tasks", format="json")]
async fn get_tasks_list() -> Result<Json<Option<Task>>, Status> {
    let task = fetch_tasks().await;
    println!("Found task:");
    println!("task {:?}", task);

    match task {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![get_books_list])
        .mount("/", routes![get_tasks_list])
}