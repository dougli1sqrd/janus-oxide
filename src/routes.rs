use std::str;
use std::convert::TryFrom;

use rocket::http::RawStr;
use rocket::request::{FromFormValue, FromSegments};
use rocket::response::status;
use rocket::State;
use rocket_contrib::json;
use rocket::http::uri::Segments;

use oxigraph::SledStore as Store;
use oxigraph::model::{NamedNode};

use crate::api::{UriWrapper, GraphType, KnownGraphType, GraphData, GraphList};
use crate::api::storage::{load_turtle_into_new_graph, read_graph_as_ttl_string, accounted_graph_list};
use crate::meta;


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

#[get("/graph?<graph_type>")]
pub fn graphs(store: State<Store>, graph_type: Option<GraphType>) -> json::Json<GraphList> {
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
pub fn add_new_graph_by_ttl(store: State<Store>, graph_uri: UriWrapper, graph_type: GraphType, triples: Vec<u8>) -> Result<json::JsonValue, status::BadRequest<String>> {
    println!("loading into {:?}", graph_uri);

    let existing_graphs = accounted_graph_list(&store);

    if existing_graphs.graphs.into_iter().any(|g| g.id == graph_uri.0.to_string() ) {
       return Err(status::BadRequest(Some(format!("Graph URI {} already exists!", graph_uri.0))))
    } else if graph_uri.0.to_string() == meta::meta_graph_uri().to_string() 
            || graph_uri.0.to_string() == meta::meta_ontology_uri().to_string() {
        
        return Err(status::BadRequest(Some("Untouchable graph".to_owned())));
    }
    
    let loaded = load_turtle_into_new_graph(&store, graph_uri.0, graph_type, triples);
    Ok(rocket_contrib::json!({"loaded": loaded}))
}

#[get("/graph/<graph_uri..>")]
pub fn get_graph(store: State<Store>, graph_uri: UriWrapper) -> Result<String, status::NotFound<String>> {
    
    let all_graphs = accounted_graph_list(&store);
    
    match all_graphs.graphs
        .into_iter()
        .find(|g: &GraphData| g.id == graph_uri.0.to_string())
        .map(|_| read_graph_as_ttl_string(&store, graph_uri.0.clone())) {
        
        Some(content) => Ok(content.unwrap()),
        None => Err(status::NotFound(format!("Graph {} cannot be found!", graph_uri.0)))
    }
}