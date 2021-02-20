#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use oxigraph::SledStore as Store;

fn prelaunch() -> Store {
    let store = Store::open("data").unwrap();
    api::storage::init(&store);
    store
}

#[get("/")]
fn index() -> &'static str {
    "Hello, World!"
}

mod api;
mod routes;
pub mod meta;

fn main() {
    println!("Hello, world!");

    let store = prelaunch();

    rocket::ignite()
        .manage(store)
        .mount("/", routes![index, routes::graphs, routes::get_graph, routes::add_new_graph_by_ttl])
        .launch();
}
