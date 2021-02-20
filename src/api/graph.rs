use std::convert::TryFrom;

use oxigraph::model::{NamedNodeRef, NamedNode};

use serde::Serialize;
use sophia_api::term::SimpleIri;

use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;

use unicase::UniCase;

use crate::meta;


#[derive(Serialize, Clone, Copy, Debug, PartialEq, EnumIter, AsRefStr)]
pub enum GraphType {
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
    pub fn uri(&self) -> SimpleIri {
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

#[derive(Debug, PartialEq)]
pub enum KnownGraphType<G> {
    Known(G),
    Unknown,
}

impl KnownGraphType<GraphType> {
    pub fn new(graph_type: GraphType) -> KnownGraphType<GraphType> {
        if graph_type == GraphType::Unknown {
            KnownGraphType::Unknown
        } else {
            KnownGraphType::Known(graph_type)
        }
    }
}

#[derive(Serialize, Debug)]
pub struct GraphData {
    pub id: String,
    pub graph_type: GraphType,
}

#[derive(Serialize)]
pub struct GraphList {
    pub context: String,
    pub graphs: Vec<GraphData>,
}

#[derive(Debug)]
pub struct UriWrapper(pub NamedNode);


#[cfg(test)]
mod testr {
    use super::*;

    #[test]
    fn test_graphtype_uri() {
        assert_eq!(GraphType::Model.uri(), meta::Model);
        assert_eq!(GraphType::Closure.uri(), meta::Closure);
        assert_eq!(GraphType::Inferred.uri(), meta::Inferred);
        assert_eq!(GraphType::Ontology.uri(), meta::Ontology);
        assert_eq!(GraphType::Unknown.uri(), meta::Unknown);
    }

    #[test]
    fn test_any_case_graph_type_from_str() {
        assert_eq!(GraphType::try_from("model"), Ok(GraphType::Model));
        assert_eq!(GraphType::try_from("Model"), Ok(GraphType::Model));
        assert_eq!(GraphType::try_from("MODEL"), Ok(GraphType::Model));
        assert_eq!(GraphType::try_from("Closure"), Ok(GraphType::Closure));
    }

    #[test]
    fn test_graph_type_try_from_error() {
        assert_eq!(GraphType::try_from("Blah"), Err("Blah"));
    }

    #[test]
    fn test_graph_type_from_uri() {
        assert_eq!(GraphType::from(NamedNode::from(meta::Ontology).as_ref()), GraphType::Ontology);
        assert_eq!(GraphType::from(NamedNode::from(meta::Unknown).as_ref()), GraphType::Unknown);
        assert_eq!(GraphType::from(NamedNodeRef::new_unchecked("http::www.example.com/Blah")), GraphType::Unknown);
    }

    #[test]
    fn test_known_graph_from_graphtype() {
        assert_eq!(KnownGraphType::new(GraphType::Model), KnownGraphType::Known(GraphType::Model));
        assert_eq!(KnownGraphType::new(GraphType::Unknown), KnownGraphType::Unknown);
    }
}


