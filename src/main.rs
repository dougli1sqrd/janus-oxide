#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use oxigraph::io::{GraphFormat};
use oxigraph::model::{NamedNode, Quad};
use oxigraph::store::sled::{SledConflictableTransactionError, SledTransaction};
use oxigraph::SledStore as Store;

use sophia_api::term::SimpleIri;

use std::convert::Infallible;
use std::fs::File;
use std::io::{BufReader};
use std::path;


fn prelaunch() -> Store {
    let store = Store::open("data").unwrap();

    let _ = store.transaction(|transaction: SledTransaction| {
        let meta_ont_path = path::Path::new("metadata/meta_ont.ttl");
        let meta_ont = File::open(meta_ont_path).unwrap();

        let _ = transaction.load_graph(
            BufReader::new(meta_ont),
            GraphFormat::Turtle,
            meta::meta_ontology_uri(),
            None,
        );

        let example_graph = NamedNode::new("http://www.purl.org/dougli1sqrd/models/janus-oxide/hello").unwrap();
        let example_triple = Quad::new(
            NamedNode::from(SimpleIri::new(example_graph.as_str(), Some("world")).unwrap()),
            oxigraph::model::vocab::rdf::TYPE,
            NamedNode::from(SimpleIri::new(example_graph.as_str(), Some("greeting")).unwrap()),
            example_graph.clone()
        );
        let example_metadata = meta::graph_metadata_entry(example_graph, api::GraphType::Model);

        println!("Inserting {}", example_triple);
        println!("Inserting {}", example_metadata);
        let _ = transaction.insert(example_triple.as_ref());
        let _ = transaction.insert(example_metadata.as_ref());

        Ok(()) as Result<(), SledConflictableTransactionError<Infallible>>
    });

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
