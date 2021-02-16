
use sophia_api::dataset::{Dataset, MutableDataset, DQuadSource, DResult, 
    DTerm, DResultTermSet, MDResult};
use sophia_api::dataset::SetDataset;
use sophia_api::quad::stream::{QuadSource, StreamResult};
use sophia_api::quad::streaming_mode::{ByValue, StreamedQuad};
use sophia_api::term::{TTerm, TermKind};

use oxigraph::store::sled::{SledQuadIter, SledStore, SledTransaction,
    SledUnabortableTransactionError, SledConflictableTransactionError, SledTransactionError};
use oxigraph::model::{Quad, QuadRef, NamedOrBlankNodeRef, NamedNodeRef, NamedNode, Term, 
    TermRef, GraphName, LiteralRef, Triple, BlankNode, BlankNodeRef, GraphNameRef};

use sophia_api::quad::Quad as SophiaQuad;

struct QuadTuple(Term, Term, Term, Option<Term>);

impl SophiaQuad for QuadTuple {
    type Term = Term;

    fn s(&self) -> &Self::Term {
        &self.0
    }

    fn p(&self) -> &Self::Term {
        &self.1
    }

    fn o(&self) -> &Self::Term {
        &self.2
    }

    fn g(&self) -> Option<&Self::Term> {
        self.3.as_ref()
    }
}


struct Store<S> {
    pub store: S
}

impl<S> Store<S> {
    pub fn new(store: S) -> Store<S> {
        Store { store }
    }
}


impl Dataset for Store<SledStore> {
    type Quad = ByValue<([Term; 3], Option<Term>)>;
    type Error = std::io::Error;


    fn quads(&self) -> DQuadSource<Self> {
        self.store.quads()
    }

    fn quads_with_s<'s, TS>(&'s self, s: &'s TS) -> DQuadSource<'s, Self>
        where
            TS: TTerm + ?Sized {

        self.store.quads_with_s(s)
    }

    fn quads_with_p<'s, TP>(&'s self, p: &'s TP) -> DQuadSource<'s, Self>
        where
            TP: TTerm + ?Sized {

        self.store.quads_with_p(p)
    }

    fn quads_with_o<'s, TP>(&'s self, o: &'s TP) -> DQuadSource<'s, Self>
        where
            TP: TTerm + ?Sized {

        self.store.quads_with_o(o)
    }

    fn quads_with_g<'s, TS>(&'s self, g: Option<&'s TS>) -> DQuadSource<'s, Self>
        where
            TS: TTerm + ?Sized {

        self.store.quads_with_g(g)
    }

    fn quads_with_sp<'s, TS, TP>(&'s self, s: &'s TS, p: &'s TP) -> DQuadSource<'s, Self>
        where
            TS: TTerm + ?Sized,
            TP: TTerm + ?Sized, {

        self.store.quads_with_sp(s, p)
    }

    fn quads_with_so<'s, TS, TO>(&'s self, s: &'s TS, o: &'s TO) -> DQuadSource<'s, Self>
        where
            TS: TTerm + ?Sized,
            TO: TTerm + ?Sized {

        self.store.quads_with_so(s, o)
    }

    fn quads_with_sg<'s, TS, TG>(&'s self, s: &'s TS, g: Option<&'s TG>) -> DQuadSource<'s, Self>
        where
            TS: TTerm + ?Sized,
            TG: TTerm + ?Sized, {

        self.store.quads_with_sg(s, g)
    }

    fn quads_with_po<'s, TP, TO>(&'s self, p: &'s TP, o: &'s TO) -> DQuadSource<'s, Self>
        where
            TP: TTerm + ?Sized,
            TO: TTerm + ?Sized,
    {
        self.store.quads_with_po(p, o)
    }

    fn quads_with_pg<'s, TP, TG>(&'s self, p: &'s TP, g: Option<&'s TG>) -> DQuadSource<'s, Self>
        where
            TP: TTerm + ?Sized,
            TG: TTerm + ?Sized,
    {
        self.store.quads_with_pg(p, g)
    }

    fn quads_with_og<'s, TO, TG>(&'s self, o: &'s TO, g: Option<&'s TG>) -> DQuadSource<'s, Self>
        where
            TO: TTerm + ?Sized,
            TG: TTerm + ?Sized,
    {
        self.store.quads_with_og(o, g)
    }

    fn quads_with_spo<'s, TS, TP, TO>(&'s self, s: &'s TS, p: &'s TP, o: &'s TO) -> DQuadSource<'s, Self>
    where
        TS: TTerm + ?Sized,
        TP: TTerm + ?Sized,
        TO: TTerm + ?Sized,
    {
        self.store.quads_with_spo(s, p, o)
    }

    fn quads_with_spg<'s, TS, TP, TG>(
        &'s self,
        s: &'s TS,
        p: &'s TP,
        g: Option<&'s TG>,
    ) -> DQuadSource<'s, Self>
    where
        TS: TTerm + ?Sized,
        TP: TTerm + ?Sized,
        TG: TTerm + ?Sized,
    {
        self.store.quads_with_spg(s, p, g)
    }

    fn quads_with_sog<'s, TS, TO, TG>(
        &'s self,
        s: &'s TS,
        o: &'s TO,
        g: Option<&'s TG>,
    ) -> DQuadSource<'s, Self>
    where
        TS: TTerm + ?Sized,
        TO: TTerm + ?Sized,
        TG: TTerm + ?Sized,
    {
        self.store.quads_with_sog(s, o, g)
    }

    fn quads_with_pog<'s, TP, TO, TG>(
        &'s self,
        p: &'s TP,
        o: &'s TO,
        g: Option<&'s TG>,
    ) -> DQuadSource<'s, Self>
    where
        TP: TTerm + ?Sized,
        TO: TTerm + ?Sized,
        TG: TTerm + ?Sized,
    {
        self.store.quads_with_pog(p, o, g)
    }

    fn quads_with_spog<'s, TS, TP, TO, TG>(
        &'s self,
        s: &'s TS,
        p: &'s TP,
        o: &'s TO,
        g: Option<&'s TG>,
    ) -> DQuadSource<'s, Self>
    where
        TS: TTerm + ?Sized,
        TP: TTerm + ?Sized,
        TO: TTerm + ?Sized,
        TG: TTerm + ?Sized,
    {
        self.store.quads_with_spog(s, p, o, g)
    }

    // TODO impl contains

    fn subjects(&self) -> DResultTermSet<Self>
        where
            DTerm<Self>: Clone + Eq + std::hash::Hash,
    {
        self.store.subjects()
    }

    fn predicates(&self) -> DResultTermSet<Self>
        where
            DTerm<Self>: Clone + Eq + std::hash::Hash,
    {
        self.store.predicates()
    }

    fn objects(&self) -> DResultTermSet<Self>
        where
            DTerm<Self>: Clone + Eq + std::hash::Hash,
    {
        self.store.objects()
    }

    fn graph_names(&self) -> DResultTermSet<Self>
        where
            DTerm<Self>: Clone + Eq + std::hash::Hash,
    {
        self.store.graph_names()
    }

    fn iris(&self) -> DResultTermSet<Self>
        where
            DTerm<Self>: Clone + Eq + std::hash::Hash,
    {
        self.store.iris()
    }

    fn bnodes(&self) -> DResultTermSet<Self>
        where
            DTerm<Self>: Clone + Eq + std::hash::Hash,
    {
        self.store.bnodes()
    }

    fn literals(&self) -> DResultTermSet<Self>
        where
            DTerm<Self>: Clone + Eq + std::hash::Hash,
    {
        self.store.literals()
    }

    fn variables(&self) -> DResultTermSet<Self>
        where
            DTerm<Self>: Clone + Eq + std::hash::Hash,
    {
        self.store.variables()
    }
}

fn convert_subject<'a, T>(term: &'a T, buffer: &'a mut String) -> Option<NamedOrBlankNodeRef<'a>>
where
    T: TTerm + ?Sized + 'a,
{
    match term.kind() {
        TermKind::Iri => Some(convert_iri(term, buffer).into()),
        TermKind::BlankNode => Some(BlankNodeRef::new_unchecked(term.value_raw().0).into()),
        _ => None,
    }
}

fn convert_predicate<'a, T>(term: &'a T, buffer: &'a mut String) -> Option<NamedNodeRef<'a>>
where
    T: TTerm + ?Sized + 'a,
{
    match term.kind() {
        TermKind::Iri => Some(convert_iri(term, buffer)),
        _ => None,
    }
}

fn convert_object<'a, T>(term: &'a T, buffer: &'a mut String) -> Option<TermRef<'a>>
where
    T: TTerm + ?Sized + 'a,
{
    match term.kind() {
        TermKind::Iri => Some(convert_iri(term, buffer).into()),
        TermKind::BlankNode => Some(BlankNodeRef::new_unchecked(term.value_raw().0).into()),
        TermKind::Literal => {
            let value = term.value_raw().0;
            let lit = match term.language() {
                Some(tag) => LiteralRef::new_language_tagged_literal_unchecked(value, tag),
                None => {
                    let (ns, suffix) = term.datatype().unwrap().destruct();
                    let datatype = convert_iri_raw(ns, suffix, buffer);
                    LiteralRef::new_typed_literal(value, datatype)
                }
            };
            Some(lit.into())
        }
        _ => None,
    }
}

fn convert_graph_name<'a, T>(
    graph_name: Option<&'a T>,
    buffer: &'a mut String,
) -> Option<GraphNameRef<'a>>
where
    T: TTerm + ?Sized + 'a,
{
    match graph_name {
        None => Some(GraphNameRef::DefaultGraph),
        Some(term) => match term.kind() {
            TermKind::Iri => Some(convert_iri(term, buffer).into()),
            TermKind::BlankNode => Some(BlankNodeRef::new_unchecked(term.value_raw().0).into()),
            _ => None,
        },
    }
}

fn convert_iri<'a, T>(term: &'a T, buffer: &'a mut String) -> NamedNodeRef<'a>
where
    T: TTerm + ?Sized + 'a,
{
    debug_assert_eq!(term.kind(), TermKind::Iri);
    let raw = term.value_raw();
    convert_iri_raw(raw.0, raw.1, buffer)
}

fn convert_iri_raw<'a>(
    ns: &'a str,
    suffix: Option<&'a str>,
    buffer: &'a mut String,
) -> NamedNodeRef<'a> {
    let iri: &'a str = match suffix {
        Some(suffix) => {
            buffer.clear();
            buffer.push_str(ns);
            buffer.push_str(suffix);
            buffer
        }
        None => ns,
    };
    NamedNodeRef::new_unchecked(iri)
}

fn make_quad_from_spog<S, P, O, G>(s: &S, p: &P, o: &O, g: Option<&G>) -> Option<Quad>
    where
        S: TTerm + ?Sized,
        P: TTerm + ?Sized,
        O: TTerm + ?Sized,
        G: TTerm + ?Sized {

    let mut buf_s = String::new();
    let mut buf_p = String::new();
    let mut buf_o = String::new();
    let mut buf_g = String::new();

    convert_subject(s, &mut buf_s)
        .and_then(
            |ss| convert_predicate(p, &mut buf_p)
        .and_then(
            |pp| convert_object(o, &mut buf_o)
        .and_then(
            |oo| convert_graph_name(g, &mut buf_g)
        .map(
            |gg| Quad::new(ss, pp, oo, gg)
    ))))
}

#[derive(Debug)]
enum MutableDatasetError {
    Io(std::io::Error),
    Conflict
}

impl std::fmt::Display for MutableDatasetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for MutableDatasetError { }

impl From<SledUnabortableTransactionError> for MutableDatasetError {
    fn from(transaction_error: SledUnabortableTransactionError) -> MutableDatasetError {
        match transaction_error {
            SledUnabortableTransactionError::Conflict => MutableDatasetError::Conflict,
            SledUnabortableTransactionError::Storage(io) => MutableDatasetError::Io(io)
        }
    }
}

impl SetDataset for Store<SledStore> {}

impl MutableDataset for Store<SledStore> {
    type MutationError = std::io::Error;

    fn insert<S, P, O, G>(&mut self, s: &S, p: &P, o: &O, g: Option<&G>) -> MDResult<Self, bool>
        where
            S: TTerm + ?Sized,
            P: TTerm + ?Sized,
            O: TTerm + ?Sized,
            G: TTerm + ?Sized {

        // let quad_opt = make_quad_from_spog(s, p, o, g).as_ref();
        let result = self.store.transaction(|transaction: SledTransaction| {
            match make_quad_from_spog(s, p, o, g).as_ref() {
                Some(quad) => {
                    match self.store.contains(quad) {
                        Ok(contained) => {
                            if !contained {
                                match transaction.insert(quad) {
                                    Ok(()) => 
                                        Ok(1),
                                    Err(e) => match e {
                                        // Err(SledConflictableTransactionError::Abort(String::from("hello")))
                                        SledUnabortableTransactionError::Storage(io) => 
                                            Err(SledConflictableTransactionError::Storage(io)) as Result<u32, SledConflictableTransactionError<String>>,
                                        SledUnabortableTransactionError::Conflict => 
                                            Err(SledConflictableTransactionError::Conflict) as Result<u32, SledConflictableTransactionError<String>>
                                    }
                                }
                            } else {
                                Ok(0)
                            }
                        },
                        Err(e) => Err(SledConflictableTransactionError::Storage(e)) as Result<u32, SledConflictableTransactionError<String>>
                    }
                },
                None => Ok(0)
            }
        });
        // Investigate the result of the transaction closure
        match result {
            Ok(0) => Ok(false),
            Ok(1) => Ok(true),
            Err(e) => match e {
                SledTransactionError::Abort(_) => Ok(false),
                SledTransactionError::Storage(io) => Err(io)
            }
            _ => Ok(false)
        }
    }

    fn insert_all<S>(&mut self, src: S) -> StreamResult<usize, S::Error, std::io::Error>
        where
            S: QuadSource {

        let result = self.store.transaction(|transaction: SledTransaction| {
            let mut inserted: usize = 0;
            let foo = src.try_for_each_quad(|q| {
                match make_quad_from_spog(q.s(), q.p(), q.o(), q.g()).as_ref() {
                    Some(quad) => {
                        match self.store.contains(quad) {
                            Ok(contained) => {
                                if !contained {
                                    match transaction.insert(quad) {
                                        Ok(()) => {
                                            inserted += 1;
                                            Ok(())
                                        },
                                        Err(err) => MutableDatasetError::from(err)
                                    }
                                } else {
                                    Ok(())
                                }
                            },
                            Err(e) => Err(SledConflictableTransactionError::Storage(e))
                        }
                    },
                    // If a streamed quad can't be converted for some reason, we'll just keep going
                    None => Ok(())
                }
                
            });
            Ok(0) as Result<u32, SledConflictableTransactionError<String>>
        });

        StreamResult::Ok(10)
    }

    fn remove<S, P, O, G>(&mut self, s: &S, p: &P, o: &O, g: Option<&G>) -> MDResult<Self, bool> 
        where
            S: TTerm + ?Sized,
            P: TTerm + ?Sized,
            O: TTerm + ?Sized,
            G: TTerm + ?Sized {

        let result = self.store.transaction(|transaction: SledTransaction| {
            match make_quad_from_spog(s, p, o, g).as_ref() {
                Some(quad) => {
                    match transaction.remove(quad) {
                        Ok(()) => Ok(true),
                        Err(err) => Err(SledConflictableTransactionError::from(err)) as Result<bool, SledConflictableTransactionError<String>>
                    }
                },
                None => Ok(false)
            }
        });
        match result {
            Ok(v) => Ok(v),
            Err(err) => match err {
                SledTransactionError::Abort(_) => Ok(false),
                SledTransactionError::Storage(io) => Err(io)
            }
        }
    }
}

