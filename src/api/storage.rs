use std::io::Cursor;
use std::convert::{Infallible};
use std::collections::HashMap;

use oxigraph::SledStore as Store;
use oxigraph::store::sled::{SledConflictableTransactionError, SledQuadIter, SledTransaction};
use oxigraph::model::{GraphNameRef, NamedNode, Quad, Triple, Term, NamedOrBlankNode};
use oxigraph::io::{GraphFormat, GraphSerializer, GraphParser};

use crate::meta;
use crate::api::{GraphType, GraphList, GraphData};

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