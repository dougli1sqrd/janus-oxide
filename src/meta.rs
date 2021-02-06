use sophia_api::namespace;

use oxigraph::model::{GraphNameRef, NamedNode, NamedNodeRef, Quad};

use crate::api::GraphType;


pub fn graph_metadata_entry(graph: NamedNode, graph_type: GraphType) -> Quad {
    Quad::new(graph, oxigraph::model::vocab::rdf::TYPE, NamedNode::from(graph_type.uri()), meta_graph_uri())
}

pub fn meta_ontology_uri() -> GraphNameRef<'static> {
    GraphNameRef::NamedNode(
        NamedNodeRef::new("http://www.purl.org/dougli1sqrd/models/janus-oxide/MetaOnt").unwrap(),
    )
}

pub fn meta_graph_uri() -> GraphNameRef<'static> {
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