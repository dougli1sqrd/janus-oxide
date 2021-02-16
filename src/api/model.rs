/// Model
///
/// 

use sophia_api::dataset::Dataset;

use oxigraph::MemoryStore;
use oxigraph::store::memory::{MemoryTransaction};
use oxigraph::SledStore;
use oxigraph::store::sled::SledTransaction;
use oxigraph::store::sled::{SledTransactionError, SledConflictableTransactionError, SledUnabortableTransactionError};
use oxigraph::io::{GraphFormat, DatasetFormat};
use oxigraph::model::{GraphNameRef, QuadRef};

use std::io::BufRead;

use std::marker::PhantomData;


struct Store<S: Dataset> {
    store: S
}

pub struct TransactStore<S, F, T, R, I, E> 
    where 
        F: Fn(T) -> Result<R, I>,
        T: Transaction, 
        S: Transact<F, T, R, I, E> {
    
    _func: PhantomData<F>,
    _trasaction: PhantomData<T>,
    _return: PhantomData<R>,
    _inner: PhantomData<I>,
    _error: PhantomData<E>,

    pub store: S
}

impl<S, F, T, R, I, E> TransactStore<S, F, T, R, I, E> 
    where 
        F: Fn(T) -> Result<R, I>,
        T: Transaction, 
        S: Transact<F, T, R, I, E> {

    pub fn new(store: S) -> TransactStore<S, F, T, R, I, E> {
        TransactStore {
            _func: PhantomData,
            _trasaction: PhantomData,
            _return: PhantomData,
            _inner: PhantomData,
            _error: PhantomData,
            store
        }
    }
}

/// The closure passed in is in the wrapped type, T and E
/// The function `transact` is in the wrapper, so it returns TransactionError.
/// 
pub trait Transact<F, T: Transaction, R, I, E> 
    where 
        F: Fn(T) -> Result<R, I> {

    fn transact(&self, f: F) -> Result<R, TransactionError<E>>;
}

/// E should be from inside SledTransactionError
impl<F, SledE> Transact<F, SledTransaction<'_>, (), SledConflictableTransactionError<SledE>, SledE> for SledStore 
    where
        F: Fn(SledTransaction) -> Result<(), SledConflictableTransactionError<SledE>> {
    
    fn transact(&self, f: F) -> Result<(), TransactionError<SledE>> {
        // f: SledTransaction -> Result<(), SledConflictableTransactionError<E>>
        let r: Result<(), SledTransactionError<SledE>> = self.transaction(f);
        match r {
            Ok(_) => Ok(()),
            Err(e) => Err(TransactionError::from(e))
        }
    }
}

pub enum TransactionError<E> {
    Aborted(E),
    OtherError(String)
}

impl<E> From<SledTransactionError<E>> for TransactionError<E> {
    fn from(e: SledTransactionError<E>) -> TransactionError<E> {
        match e {
            SledTransactionError::Abort(a) => TransactionError::Aborted(a),
            SledTransactionError::Storage(s) => TransactionError::OtherError(format!("{}", s))
        }
    }
}


// impl<E> From<SledTransactionError<E>> for TransactionResultError {
//     fn from(_: SledTransactionError<E>) -> TransactionResultError {
//         TransactionResultError::Error
//     }
// }

pub trait Transaction {
    fn load_graph<'a>(&self,
        reader: impl BufRead,
        format: GraphFormat,
        graph_name: impl Into<GraphNameRef<'a>>,
        base_iri: Option<&str>) -> Result<(), TransactionOpError>;

    fn load_dataset(&self,
        reader: impl BufRead,
        format: DatasetFormat,
        base_iri: Option<&str>) -> Result<(), TransactionOpError>;
    
    fn insert<'a>(&self, quad: impl Into<QuadRef<'a>>) -> Result<(), TransactionOpError>;

    fn remove<'a>(&self, quad: impl Into<QuadRef<'a>>) -> Result<(), TransactionOpError>;
}

pub enum TransactionOpError {
    Unabortable
}

impl From<SledUnabortableTransactionError> for TransactionOpError {
    fn from(_: SledUnabortableTransactionError) -> TransactionOpError {
        TransactionOpError::Unabortable
    }
}

impl Transaction for SledTransaction<'_> {
    fn load_graph<'a>(&self,
        reader: impl BufRead,
        format: GraphFormat,
        graph_name: impl Into<GraphNameRef<'a>>,
        base_iri: Option<&str>) -> Result<(), TransactionOpError> {
    
        match self.load_graph(reader, format, graph_name, base_iri) {
            Ok(_) => Ok(()),
            Err(e) => Err(TransactionOpError::from(e))
        }
    }

    fn load_dataset(&self,
        reader: impl BufRead,
        format: DatasetFormat,
        base_iri: Option<&str>) -> Result<(), TransactionOpError> {

        match self.load_dataset(reader, format, base_iri) {
            Ok(_) => Ok(()),
            Err(e) => Err(TransactionOpError::from(e))
        }
    }

    fn insert<'a>(&self, quad: impl Into<QuadRef<'a>>) -> Result<(), TransactionOpError> {
        self.insert(quad).map_err(TransactionOpError::from)
    }

    fn remove<'a>(&self, quad: impl Into<QuadRef<'a>>) -> Result<(), TransactionOpError> {
        self.remove(quad).map_err(TransactionOpError::from)
    }
}

// impl<F, T, E> Transact<F, SledTransaction<'_>, T, SledConflictableTransactionError<E>, SledTransactionError<E>> for SledStore
//     where
//         F: Fn(SledTransaction<'_>) -> Result<T, SledConflictableTransactionError<E>> {

//     fn transaction(&self, f: F) -> Result<T, SledTransactionError<E>> {
//         self.transaction(f)
//     }
// }

// impl<F, E> Transact<F, &mut MemoryTransaction, (), E, E> for MemoryStore
//     where
//         F: Fn(&mut MemoryTransaction) -> Result<(), E> {

//     fn transaction(&self, f: F) -> Result<(), E> {
//         self.transaction(f)
//     }
// }
