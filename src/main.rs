#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket::State;
use rocket_contrib::json;
use rocket::http::RawStr;
use rocket::request::FromFormValue;

use oxigraph;
use oxigraph::SledStore as Store;
use oxigraph::store::sled::{SledTransaction, SledConflictableTransactionError, SledQuadIter};
use oxigraph::io::GraphFormat;
use oxigraph::model::{GraphName, NamedNodeRef, GraphNameRef, NamedOrBlankNode, NamedOrBlankNodeRef, TermRef, Quad, NamedNode, Term};

use sophia_api::ns::Namespace;

use itertools::Itertools;

use unicase::UniCase;

use std::path;
use std::fs::File;
use std::io::BufReader;
use std::convert::{Infallible, TryFrom};
use std::collections::HashMap;
use std::str;

#[macro_use]
use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug)]
enum GraphType {
    Ontology,
    Closure,
    Model,
    Inferred,
    Unknown
}

impl<'a> TryFrom<&'a str> for GraphType {
    type Error = &'a str;

    fn try_from(val: &'a str) -> Result<GraphType, Self::Error> {
        let ont = UniCase::new("ontology");
        let close = UniCase::new("closure");
        let model = UniCase::new("model");
        let inferred = UniCase::new("inferred");

        match UniCase::new(val) {
            ont => Ok(GraphType::Ontology),
            close => Ok(GraphType::Closure),
            model => Ok(GraphType::Model),
            inferred => Ok(GraphType::Inferred),
            _ => Err(val)
        }
    }
}

impl<'v> FromFormValue<'v> for GraphType {
    type Error = &'v RawStr;

    fn from_form_value(form_value: &'v RawStr) -> Result<GraphType, &'v RawStr> {
        let slice = str::from_utf8(form_value.as_bytes());
        match slice {
            Ok(s) => match GraphType::try_from(s) {
                Ok(g) => Ok(g),
                Err(_) => Err(form_value)
            },
            Err(_) => Err(form_value)
        }
    }
}

#[derive(Serialize)]
struct GraphData {
    id: String,
    graph_type: GraphType,
}

#[derive(Serialize)]
struct GraphList {
    context: String,
    graphs: Vec<GraphData>
}

#[get("/")]
fn index() -> &'static str {
    "Hello, World!"
}

#[get("/graph?<graph_type>")]
fn graphs(store: State<Store>, graph_type: Option<GraphType>) -> json::Json<GraphList> {
    println!("got type {:?}", graph_type);
    
    json::Json(accounted_graph_list(&store))
}

fn accounted_graph_list(store: &Store) -> GraphList {
    let iter = store.quads_for_pattern(None, None, None, Some(meta_ontology_uri()));
    let mut subject_map = map_by_subject(iter);
    let mut graphs: Vec<GraphData> = vec![];

    for (graph_name, po_list) in subject_map.into_iter() {
        let g_str =match po_list.iter().find(|(p, _)| {
            p.as_ref() == oxigraph::model::vocab::rdf::TYPE
        }) {
            Some((_, o)) => match o {
                Term::NamedNode(n) => n.as_str(),
                _ => "Unknown"
            },
            None => "Unknown"
        };

        // TODO This is gross, ew, apologies
        let gt = match g_str {
            "<http://www.purl.org/dougli1sqrd/models/janus-oxide/meta/Ontology>" => GraphType::Ontology,
            "<http://www.purl.org/dougli1sqrd/models/janus-oxide/meta/Closure>" => GraphType::Closure,
            "<http://www.purl.org/dougli1sqrd/models/janus-oxide/meta/Model>" => GraphType::Model,
            "<http://www.purl.org/dougli1sqrd/models/janus-oxide/meta/Inferred>" => GraphType::Inferred,
            _ => GraphType::Unknown
        };

        graphs.push(GraphData {
            id: graph_name.to_string(),
            graph_type: gt
        });
    }

    GraphList {
        context: "http://www.purl.org/dougli1sqrd/models/janus-oxide/meta/context.json".into(),
        graphs
    }
}

fn map_by_subject(iter: SledQuadIter) -> HashMap<NamedOrBlankNode, Vec<(NamedNode, Term)>> {
    iter.fold(HashMap::new(), |mut current_map, quad_res| {
        let quad: Quad = quad_res.unwrap();
        let i = current_map.entry(quad.subject).or_insert(vec![]);
        i.push((quad.predicate, quad.object));
        current_map
    })
}

fn meta_ontology_uri() -> GraphNameRef<'static> {
    GraphNameRef::NamedNode(NamedNodeRef::new("http://www.purl.org/dougli1sqrd/models/janus-oxide/MetaOnt").unwrap())
}

fn meta_graph_uri() -> GraphNameRef<'static> {
    GraphNameRef::NamedNode(NamedNodeRef::new("http://www.purl.org/dougli1sqrd/models/janus-oxide/Meta").unwrap())
}

fn prelaunch() -> Store {
    let store = Store::open("data").unwrap();

    let _ = store.transaction(|transaction: SledTransaction| {
        let meta_ont_path = path::Path::new("metadata/meta_ont.ttl");
        let meta_ont = File::open(meta_ont_path).unwrap();
    
        let _ = transaction.load_graph(BufReader::new(meta_ont), GraphFormat::Turtle, meta_ontology_uri(), None);
        Ok(()) as Result<(), SledConflictableTransactionError<Infallible>>
    });

    store
}


fn main() {
    println!("Hello, world!");

    let store = prelaunch();

    rocket::ignite()
        .manage(store)
        .mount("/", routes![index, graphs]).launch();
}
