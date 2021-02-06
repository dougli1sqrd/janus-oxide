#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket::http::RawStr;
use rocket::request::{FromFormValue, FromSegments};
use rocket::data::FromDataSimple;
use rocket::response::status;
use rocket::State;
use rocket_contrib::json;
use rocket::http::uri::Segments;

use oxigraph::io::{GraphFormat, GraphSerializer, GraphParser};
use oxigraph::model::{GraphNameRef, NamedNode, NamedNodeRef, NamedOrBlankNode, Quad, Triple, Term};
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
use std::io::{BufReader, Cursor};
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
        match GraphType::iter().find(|g| NamedNode::from(g.uri()).as_ref() == uri) {
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

#[derive(Serialize, Debug)]
struct GraphData {
    id: String,
    graph_type: GraphType,
}

#[derive(Serialize)]
struct GraphList {
    context: String,
    graphs: Vec<GraphData>,
}

#[derive(Debug)]
struct UriWrapper(NamedNode);

impl<'u> FromSegments<'u> for UriWrapper {
    type Error = &'u RawStr;

    fn from_segments(param: Segments<'u>) -> Result<UriWrapper, Self::Error> {
        let raw: &'u RawStr = RawStr::from_str(param.0);
        decode_uri(raw)
    }
}

impl<'u> FromFormValue<'u> for UriWrapper {
    type Error = &'u RawStr;

    fn from_form_value(form_value: &'u RawStr) -> Result<UriWrapper, Self::Error> {
        decode_uri(form_value)
    }
}

fn decode_uri(raw_uri: &RawStr) -> Result<UriWrapper, &RawStr> {
    let decoded = raw_uri.percent_decode().unwrap();
    if decoded.starts_with('<') && decoded.ends_with('>') {
        let unbracketed = decoded.trim_start_matches('<').trim_end_matches('>');
        match NamedNode::new(unbracketed) {
            Ok(named) => Ok(UriWrapper(named)),
            Err(_) => Err(raw_uri)
        }
    } else {
        Err(raw_uri)
    }
}


#[get("/")]
fn index() -> &'static str {
    "Hello, World!"
}

#[get("/graph?<graph_type>")]
fn graphs(store: State<Store>, graph_type: Option<GraphType>) -> json::Json<GraphList> {
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

#[post("/graph?<graph_uri>&<graph_type>", format="text/turtle", data="<triples>")]
fn add_new_graph_by_ttl(store: State<Store>, graph_uri: UriWrapper, graph_type: GraphType, triples: Vec<u8>) -> Result<json::JsonValue, status::BadRequest<String>> {
    println!("loading into {:?}", graph_uri);

    let existing_graphs = accounted_graph_list(&store);

    if existing_graphs.graphs.into_iter().any(|g| g.id == graph_uri.0.to_string() ) {
       return Err(status::BadRequest(Some(format!("Graph URI {} already exists!", graph_uri.0))))
    } else if graph_uri.0.to_string() == meta_graph_uri().to_string() 
            || graph_uri.0.to_string() == meta_ontology_uri().to_string() {
        
        return Err(status::BadRequest(Some("Untouchable graph".to_owned())));
    }
    
    let loaded = load_turtle_into_new_graph(&store, graph_uri.0, graph_type, triples);
    Ok(rocket_contrib::json!({"loaded": loaded}))
}

#[get("/graph/<graph_uri..>")]
fn get_graph(store: State<Store>, graph_uri: UriWrapper) -> Result<String, status::NotFound<String>> {
    
    let all_graphs = accounted_graph_list(&store);
    
    match all_graphs.graphs
        .into_iter()
        .find(|g: &GraphData| g.id == graph_uri.0.to_string())
        .map(|_| read_graph_as_ttl_string(&store, graph_uri.0.clone())) {
        
        Some(content) => Ok(content.unwrap()),
        None => Err(status::NotFound(format!("Graph {} cannot be found!", graph_uri.0)))
    }
}

fn load_turtle_into_new_graph(store: &Store, graph_uri: NamedNode, graph_type: GraphType, triples: Vec<u8>) -> usize {
    let metadata_entry = graph_metadata_entry(graph_uri.clone(), graph_type);

    let parser = GraphParser::from_format(GraphFormat::Turtle);

    let r: Vec<_> = parser.read_triples(Cursor::new(triples)).unwrap()
        .collect::<Result<Vec<_>,_>>().unwrap();
    
    let number_parsed = r.len();
    println!("Parsed {} triples", number_parsed);

    let _ = store.transaction(|transaction: SledTransaction| {
        let _ = transaction.insert(metadata_entry.as_ref());
        let results: Result<Vec<_>, _> = r.clone().into_iter().map(|triple| {
            transaction.insert(triple.in_graph(graph_uri.as_ref()).as_ref())
        }).collect();

        let _ = match results {
            Ok(_) => {},
            Err(e) => {
                println!("Broke when inserting! {}", e);
            }
        };

        Ok(()) as Result<(), SledConflictableTransactionError<Infallible>>
    });

    number_parsed

}

fn read_graph_as_ttl_string(store: &Store, graph_uri: NamedNode) -> Result<String, String> {
    let quads_iter = store.quads_for_pattern(None, None, None,
        Some(GraphNameRef::NamedNode(graph_uri.as_ref())));

    let mut buffer = Vec::new();
    let mut writer = GraphSerializer::from_format(GraphFormat::Turtle).triple_writer(&mut buffer).unwrap();
    quads_iter.map(|q| Triple::from(q.unwrap()))
        .fold(&mut writer, |w, triple| {
            let _ = w.write(triple.as_ref()); // Ignore error cause we're bad
            w
    });
    let _ = writer.finish();

    String::from_utf8(buffer).map_err(|e| e.to_string())
}

fn accounted_graph_list(store: &Store) -> GraphList {
    let iter = store.quads_for_pattern(None, None, None, Some(meta_graph_uri()));
    let subject_map = map_by_subject(iter);

    println!("{:?}", subject_map);
    
    let graphs = subject_map
        .into_iter()
        .map(|(graph_name, po_list)| {
            let g = match po_list
                .iter()
                .find(|(p, _)| p.as_ref() == oxigraph::model::vocab::rdf::TYPE)
            {
                Some((_, Term::NamedNode(o))) => GraphType::from(o.as_ref()),
                _ => GraphType::Unknown,
            };

            GraphData {
                id: graph_name.to_string(),
                graph_type: g,
            }
        })
        .collect();

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

fn graph_metadata_entry(graph: NamedNode, graph_type: GraphType) -> Quad {
    Quad::new(graph, oxigraph::model::vocab::rdf::TYPE, NamedNode::from(graph_type.uri()), meta_graph_uri())
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

        let example_graph = NamedNode::new("http://www.purl.org/dougli1sqrd/models/janus-oxide/hello").unwrap();
        let example_triple = Quad::new(
            NamedNode::from(SimpleIri::new(example_graph.as_str(), Some("world")).unwrap()),
            oxigraph::model::vocab::rdf::TYPE,
            NamedNode::from(SimpleIri::new(example_graph.as_str(), Some("greeting")).unwrap()),
            example_graph.clone()
        );
        let example_metadata = graph_metadata_entry(example_graph, GraphType::Model);

        println!("Inserting {}", example_triple);
        println!("Inserting {}", example_metadata);
        let _ = transaction.insert(example_triple.as_ref());
        let _ = transaction.insert(example_metadata.as_ref());

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
        .mount("/", routes![index, graphs, get_graph, add_new_graph_by_ttl])
        .launch();
}
