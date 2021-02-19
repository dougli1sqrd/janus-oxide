use std::io::{Cursor, BufReader};
use std::convert::{Infallible};
use std::collections::HashMap;
use std::path;
use std::fs::File;

use oxigraph::SledStore as Store;
use oxigraph::store::sled::{SledConflictableTransactionError, SledQuadIter, SledTransaction};
use oxigraph::model::{GraphNameRef, NamedNode, NamedNodeRef, Quad, Triple, Term, NamedOrBlankNode};
use oxigraph::io::{GraphFormat, GraphSerializer, GraphParser};

use sophia_api::term::SimpleIri;

use crate::meta;
use crate::api::{GraphType, GraphList, GraphData};

/// Load a Vec of bytes representing Turtle formatted triples into a named graph, `graph_uri`.
/// The type of the RDF data: (Model, Ontology, Inference, or Closure) needs to be also specified.
/// Returned is the number of triples parsed.
/// 
/// The bytes are parsed using the oxigraph parser, and then loaded into the Store.
/// 
/// TODO this should be updated to: 
/// 1) Report errors
/// 2) Return the number of triples *loaded*, not parsed
/// 
/// The new graph is also added as an entry in the metadata graph
pub fn load_turtle_into_new_graph(store: &Store, graph_uri: NamedNode, graph_type: GraphType, triples: Vec<u8>) -> usize {
    let metadata_entry = meta::graph_metadata_entry(graph_uri.clone(), graph_type);

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

/// Read out the triples in a named graph as Turtle.
pub fn read_graph_as_ttl_string(store: &Store, graph_uri: NamedNode) -> Result<String, String> {
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


/// Lists the named graphs loaded into the metadata triplestore graph at
/// <http://www.purl.org/dougli1sqrd/models/janus-oxide/Meta>.
/// 
/// This is read into a GraphList struct.
pub fn accounted_graph_list(store: &Store) -> GraphList {
    let iter = store.quads_for_pattern(None, None, None, Some(meta::meta_graph_uri()));
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

/// To initialize the triplestore, we will load turtle file at `metadata/meta_ont.ttl` 
/// that describes the small set of terms Janus will use to keep track of different 
/// types of graphs added to the store and their relationships. This small ontology
/// will be loaded into the graph name at `meta::meta_ontology_uri()`.
/// 
/// Then the graph that stores all the other graph metadata will be created with
/// URI from `meta::graph_metadata_entry()`. An example graph and data will created
/// to demonstrate.
/// 
/// A simple "hello world" triple will be added:
/// ```
/// @prefix : <http://www.purl.org/dougli1sqrd/models/janus-oxide/> .
/// :helloworld a :hellogreeting .
/// ```
/// to named graph `:hello`. 
/// 
/// Additionally the `:hello` graph will be added to the metadata graph by the quad:
/// ```
/// :hello a meta:Model <http://www.purl.org/dougli1sqrd/models/janus-oxide/Meta> .
/// ```
/// 
/// So there should be three actual named graphs after init:
/// 1) `MetaOnt`, where the metadata ontology is stored
/// 2) `Meta`, whre the graph metadata will be placed as more graphs are added
/// 3) `:hello`, as an example and containing a single example triple.
pub fn init(store: &Store) {
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
        let example_metadata = meta::graph_metadata_entry(example_graph, GraphType::Model);

        println!("Inserting {}", example_triple);
        println!("Inserting {}", example_metadata);
        let _ = transaction.insert(example_triple.as_ref());
        let _ = transaction.insert(example_metadata.as_ref());

        Ok(()) as Result<(), SledConflictableTransactionError<Infallible>>
    });
}

#[cfg(test)]
mod test {
    use super::*;

    // Makes a new store at a temporary directory. The Temp Dir handle is also returned.
    fn make_temp_store() -> (Store, tempfile::TempDir) {
        let tempdir = tempfile::tempdir().expect("Could not creat temporary file");
        let fpath = &tempdir.path().join("foo.txt");
        let store = Store::open(fpath).expect("Couldn't open SledStore");
        (store, tempdir)
    }

    /// Runs init() on the store and then returns it
    fn init_store() -> (Store, tempfile::TempDir) {
        let (s, f) = make_temp_store();
        init(&s);
        (s, f)
    }

    #[test]
    fn test_accounted_graph_list() {
        let (s, _f) = init_store();

        let graphs = accounted_graph_list(&s);
        assert_eq!(graphs.context, String::from("http://www.purl.org/dougli1sqrd/models/janus-oxide/meta/context.json"));
        assert_eq!(graphs.graphs.len(), 1);
    }

    #[test]
    fn test_init() {
        use std::collections::HashSet;

        let (s, _f): (Store, tempfile::TempDir) = make_temp_store();

        init(&s);

        assert!(s.contains_named_graph(NamedNode::new_unchecked("http://www.purl.org/dougli1sqrd/models/janus-oxide/Meta").as_ref()).expect("Should work"));

        let g: HashSet<_> = s.named_graphs().filter_map(Result::ok).collect();

        let mut expected = HashSet::new();
        expected.insert(NamedOrBlankNode::NamedNode(NamedNode::new_unchecked("http://www.purl.org/dougli1sqrd/models/janus-oxide/Meta")));
        expected.insert(NamedOrBlankNode::NamedNode(NamedNode::new_unchecked("http://www.purl.org/dougli1sqrd/models/janus-oxide/MetaOnt")));
        expected.insert(NamedOrBlankNode::NamedNode(NamedNode::new_unchecked("http://www.purl.org/dougli1sqrd/models/janus-oxide/hello")));

        assert_eq!(g, expected);
    }

    #[test]
    fn test_load_turtle() {
        let (s, _f): (Store, _) = init_store();

        let triple = "<http://www.example.com/A> <http://www.example.com/is> <http://www.example.com/B> .".as_bytes();
        let graph = "http://www.example.com";

        let v = load_turtle_into_new_graph(&s, NamedNode::new_unchecked(graph), GraphType::Model, triple.to_vec());

        assert_eq!(1, v);

        let accounted = accounted_graph_list(&s);
        let found = accounted.graphs.iter().find(|g| g.id == "<http://www.example.com>").unwrap();
        assert_eq!(String::from("<http://www.example.com>"), found.id);
        assert_eq!(GraphType::Model, found.graph_type);

        let quad_in_graph: Vec<_> = s.quads_for_pattern(None, None, None, Some(GraphNameRef::NamedNode(NamedNodeRef::new_unchecked(graph)))).filter_map(Result::ok).collect();
        
        let sub = NamedNodeRef::new_unchecked("http://www.example.com/A");
        let pred = NamedNodeRef::new_unchecked("http://www.example.com/is");
        let obj = NamedNodeRef::new_unchecked("http://www.example.com/B");
        let graph_node = NamedNodeRef::new_unchecked("http://www.example.com");
        assert_eq!(vec![Quad::new(sub, pred, obj, graph_node)], quad_in_graph);
    }
}