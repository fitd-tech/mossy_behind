#[macro_use] extern crate rocket;
use mongodb::results::{InsertOneResult, DeleteResult};
use mongodb::{Client, options::ClientOptions};
// use mongodb::bson::extjson::de::Error;
// use mongodb::bson::{doc, Document};
// use mongodb::Collection;
use mongodb::error::Error;
use futures::stream::TryStreamExt;
use rocket::http::Status;
use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::request::{self, Outcome, Request, FromRequest};
use mongodb::bson;

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
struct TaskData {
    name: String,
    frequency: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Task {
    _id: bson::oid::ObjectId,
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

    /* while let Some(book) = cursor.try_next().await? {
        println!("book: {:?}", book);
    } */

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

async fn get_tasks_list_action() ->Result<Vec<Task>, Error> { // Result<Vec<Task>, Error>
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

    let mut cursor = tasks.find(None, None).await?;

    let mut tasks_list = Vec::new();

    while let Some(task) = cursor.try_next().await? {
        println!("task: {:?}", task);
        tasks_list.push(task);
    }

    // let task = tasks.find_one(None, None).await;

    /* match task {
        Ok(task_result) => Ok(task_result),
        Err(_) => todo!(),
    } */
    /* match tasks_list {
        Ok(tasks_list_result) => Ok(tasks_list_result),
        Err(_) => todo!(),
    } */
    Ok(tasks_list)
}

async fn post_task_action(task_data: TaskData) -> Result<InsertOneResult, Error> { // Result<Vec<Task>, Error>
    println!("task_data {:?}", task_data);
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

    // let cursor = tasks.find(None, None).await?;

    let new_task = Task {
        _id: bson::oid::ObjectId::new(),
        name: task_data.name,
        frequency: task_data.frequency,
    };
    println!("new_task: {:?}", new_task);

    let task_result = tasks.insert_one(new_task, None).await;

    /* let mut tasks_list = Vec::new();

    while let Some(task) = cursor.try_next().await? {
        println!("task: {:?}", task);
        tasks_list.push(task);
    } */

    /* let docs = vec![    
        Book { title: "1984".to_string(), author: "George Orwell".to_string() },
        Book { title: "Animal Farm".to_string(), author: "George Orwell".to_string() },
        Book { title: "The Great Gatsby".to_string(), author: "F. Scott Fitzgerald".to_string() },
    ]; */

    // Disable insert since we have a LOT of books by now
    // We should figure out how to get a count of all records
    // books.insert_many(docs, None).await?;

    // let task = tasks.find_one(None, None).await;

    match task_result {
        Ok(_task_result) => Ok(_task_result),
        Err(_) => todo!(),
    }
    /* match tasks_list {
        Ok(tasks_list_result) => Ok(tasks_list_result),
        Err(_) => todo!(),
    } */
    // Ok(tasks_list)
}

async fn delete_tasks_action(tasks_data: Vec<bson::oid::ObjectId>) -> Result<DeleteResult, Error> {
    println!("task_data {:?}", tasks_data);
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

    /* let cursor = tasks.find(None, None).await?;

    let mut tasks_list = Vec::new();

    while let Some(task) = cursor.try_next().await? {
        println!("task: {:?}", task);
        tasks_list.push(task);
    } */

    /* let new_task = Task {
        _id: bson::oid::ObjectId::new(),
        name: task_data.name,
        frequency: task_data.frequency,
    };
    println!("new_task: {:?}", new_task); */

    let filter = bson::doc!{"_id": { "$in": tasks_data }};

    let tasks_result = tasks.delete_many(filter, None).await;

    /* let docs = vec![    
        Book { title: "1984".to_string(), author: "George Orwell".to_string() },
        Book { title: "Animal Farm".to_string(), author: "George Orwell".to_string() },
        Book { title: "The Great Gatsby".to_string(), author: "F. Scott Fitzgerald".to_string() },
    ]; */

    // Disable insert since we have a LOT of books by now
    // We should figure out how to get a count of all records
    // books.insert_many(docs, None).await?;

    // let task = tasks.find_one(None, None).await;

    match tasks_result {
        Ok(_tasks_result) => Ok(_tasks_result),
        Err(_) => todo!(),
    }
    /* match tasks_list {
        Ok(tasks_list_result) => Ok(tasks_list_result),
        Err(_) => todo!(),
    } */
    // Ok(tasks_list)
}

#[get("/api/tasks", format="json")]
async fn get_tasks_list() -> Result<Json<Vec<Task>>, Status> {
    let tasks = get_tasks_list_action().await;

    match tasks {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/api/tasks", format="json", data="<task>")]
async fn post_task(task: Json<TaskData>) -> Result<Json<InsertOneResult>, Status> {
    let deserialized_task = task.into_inner();
    println!("deserialized_task {:?}", deserialized_task);
    let task = post_task_action(deserialized_task).await;

    match task {
        Ok(task_result) => Ok(Json(task_result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/api/tasks", format="json", data="<tasks>")]
async fn delete_tasks(tasks: Json<Vec<bson::oid::ObjectId>>) -> Result<Json<DeleteResult>, Status> {
    let deserialized_tasks_list = tasks.into_inner();
    println!("deserialized_tasks {:?}", deserialized_tasks_list);
    let tasks = delete_tasks_action(deserialized_tasks_list).await;

    match tasks {
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
        .mount("/", routes![post_task])
        .mount("/", routes![delete_tasks])
}