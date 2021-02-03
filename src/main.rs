#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket::http::RawStr;
use rocket::request::FromFormValue;
use rocket::State;
use rocket_contrib::json;

use oxigraph::io::GraphFormat;
use oxigraph::model::{GraphNameRef, NamedNode, NamedNodeRef, NamedOrBlankNode, Quad, Term};
use oxigraph::store::sled::{SledConflictableTransactionError, SledQuadIter, SledTransaction};
use oxigraph::SledStore as Store;

use sophia_api::term::SimpleIri;
use sophia_api::term::TTerm;
use sophia_api::term::TryCopyTerm;

use itertools::Itertools;

use unicase::UniCase;

use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;

use std::collections::HashMap;
use std::convert::{AsRef, Infallible, TryFrom};
use std::fs::File;
use std::io::BufReader;
use std::path;
use std::str;

#[macro_use]
use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, EnumIter, AsRefStr)]
enum GraphType {
    Ontology,
    Closure,
    Model,
    Inferred,
    Unknown,
}

///
/// For readability and writeability, we will use the `meta` module as the rust
/// namespace of controlled vocabulary defined in `metadata/meta_ont.ttl`. This uses
/// the sophia_api crate.
///
/// When reading URIs from the graph, these will come in from the Oxigraph models.
/// So to get to a GraphType variant, we will first convert an Oxigraph `NamedNode`
/// to a `SimpleIri` from sophia. Then we can convert from a sohpia SimpleIri into
/// a GraphType.
///
/// NamedNode -> SimpleIri -> GraphType
///
/// GraphType -> SimpleIri
///
impl GraphType {
    fn uri(&self) -> SimpleIri {
        match self {
            GraphType::Ontology => meta::Ontology,
            GraphType::Closure => meta::Closure,
            GraphType::Model => meta::Model,
            GraphType::Inferred => meta::Inferred,
            GraphType::Unknown => meta::Unknown,
        }
    }
}

impl<'a> TryFrom<&'a str> for GraphType {
    type Error = &'a str;

    fn try_from(val: &'a str) -> Result<GraphType, Self::Error> {
        // Use Unicase to test fuzzy equality case insensitively
        let c = UniCase::new(val);
        match GraphType::iter().find(|g| UniCase::new(g.as_ref()) == c) {
            Some(g) => Ok(g),
            None => Err(val),
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
                Err(_) => Err(form_value),
            },
            Err(_) => Err(form_value),
        }
    }
}

// ///
// /// Thin wrapper around NamedNode to allow us to make trait impls on NamedNode
// struct OxiUri(NamedNode);

// /// As written this takes `SimpleIri` and brings them to `NamedNode` in a local wrapper.
// /// This means SimpleIri can be into() -> OxiUri
// impl<'s> From<SimpleIri<'_>> for OxiUri {
//     fn from(iri: SimpleIri) -> OxiUri {
//         let inner = iri.to_string();
//         // It's okay to unwrap here since it was already known to be correctly parsed in SimpleIri
//         OxiUri(NamedNode::new(inner).unwrap())
//     }
// }

/// This converts an OxiGraph `NamedNode` into a GraphType. This employs the sophia
/// `TTerm` trait which allows equality tests between different types that implement
/// the trait. We have the `sophia` feature turned on for the oxigraph crate dependency
/// which provides those implementations of `TTerm` for oxigraph types.
///
/// This will iterate through all simple variants of GraphType, getting the associated
/// `uri()` method to get the SimpleIri which is then compared. If the two are equal,
/// we use that GraphType variant.
///
/// It's not ideal to need to iterate through the variants. It's possible there's a `lazy_static!`
/// way to associate these two values together so iteration isn't needed. Luckily the set is small
/// so the actual overhead should be small.
impl<'n> From<NamedNodeRef<'n>> for GraphType {
    fn from(uri: NamedNodeRef) -> GraphType {
        match GraphType::iter().find(|g| g.uri() == uri) {
            Some(g) => g,
            None => GraphType::Unknown,
        }
    }
}

enum KnownGraphType<G> {
    Known(G),
    Unknown,
}

impl KnownGraphType<GraphType> {
    fn new(graph_type: GraphType) -> KnownGraphType<GraphType> {
        if graph_type == GraphType::Unknown {
            KnownGraphType::Unknown
        } else {
            KnownGraphType::Known(graph_type)
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
    graphs: Vec<GraphData>,
}

#[get("/")]
fn index() -> &'static str {
    "Hello, World!"
}

#[get("/graph?<graph_type>")]
fn graphs(store: State<Store>, graph_type: Option<GraphType>) -> json::Json<GraphList> {
    println!("got type {:?}", graph_type);

    let graphs = accounted_graph_list(&store);
    if let Some(KnownGraphType::Known(g)) = graph_type.map(KnownGraphType::new) {
        let filtered_graphs: Vec<GraphData> = graphs
            .graphs
            .into_iter()
            .filter(|data| data.graph_type == g)
            .collect();
        json::Json(GraphList {
            context: graphs.context,
            graphs: filtered_graphs,
        })
    } else {
        json::Json(accounted_graph_list(&store))
    }
}

fn accounted_graph_list(store: &Store) -> GraphList {
    let iter = store.quads_for_pattern(None, None, None, Some(meta_graph_uri()));
    let subject_map = map_by_subject(iter);
    let mut graphs: Vec<GraphData> = vec![];

    for (graph_name, po_list) in subject_map.into_iter() {
        let g = match po_list
            .iter()
            .find(|(p, _)| p.as_ref() == oxigraph::model::vocab::rdf::TYPE)
        {
            Some((_, Term::NamedNode(o))) => GraphType::from(o.as_ref()),
            _ => GraphType::Unknown,
        };

        graphs.push(GraphData {
            id: graph_name.to_string(),
            graph_type: g,
        });
    }

    GraphList {
        context: "http://www.purl.org/dougli1sqrd/models/janus-oxide/meta/context.json".into(),
        graphs,
    }
}

///
/// Takes an iterator of Quads and and groups them by shared Subject, to produce a map of entries
/// of the subject node to a list of (predicate, object) tuples that all have the same subject.
/// This map is returned
fn map_by_subject(iter: SledQuadIter) -> HashMap<NamedOrBlankNode, Vec<(NamedNode, Term)>> {
    iter.fold(HashMap::new(), |mut current_map, quad_res| {
        let quad: Quad = quad_res.unwrap();
        let i = current_map.entry(quad.subject).or_insert_with(Vec::new);
        i.push((quad.predicate, quad.object));
        current_map
    })
}

fn meta_ontology_uri() -> GraphNameRef<'static> {
    GraphNameRef::NamedNode(
        NamedNodeRef::new("http://www.purl.org/dougli1sqrd/models/janus-oxide/MetaOnt").unwrap(),
    )
}

fn meta_graph_uri() -> GraphNameRef<'static> {
    // This should somehow be using the `meta` module below.
    // But in order to move between SimpleIRI (from sophia_api) we need to either define
    // a convert (`From`) or `PartialEq`.
    // But 1) We need to wrap either the oxigraph types or wrap `SimpleIRI` so we can impl
    // traits on them
    // And 2) What the hell which part of the oxigraph model do we target? Refs? The enums?
    // The underlying structs inside each enum variant? Confusing.
    GraphNameRef::NamedNode(
        NamedNodeRef::new("http://www.purl.org/dougli1sqrd/models/janus-oxide/Meta").unwrap(),
    )
}

fn prelaunch() -> Store {
    let store = Store::open("data").unwrap();

    let _ = store.transaction(|transaction: SledTransaction| {
        let meta_ont_path = path::Path::new("metadata/meta_ont.ttl");
        let meta_ont = File::open(meta_ont_path).unwrap();

        let _ = transaction.load_graph(
            BufReader::new(meta_ont),
            GraphFormat::Turtle,
            meta_ontology_uri(),
            None,
        );
        Ok(()) as Result<(), SledConflictableTransactionError<Infallible>>
    });

    store
}

pub mod meta {
    use sophia_api::namespace;

    namespace!(
        "http://www.purl.org/dougli1sqrd/models/janus-oxide/Meta/",
        // classes
        Graph,
        Ontology,
        Closure,
        Model,
        Inferred,
        Unknown,
        // relations
        inferredFrom,
        hasInferencesAt
    );
}

fn main() {
    println!("Hello, world!");

    let store = prelaunch();

    rocket::ignite()
        .manage(store)
        .mount("/", routes![index, graphs])
        .launch();
}
