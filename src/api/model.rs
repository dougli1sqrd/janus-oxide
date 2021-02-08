/// Model
///
/// 

use sophia_api::dataset::Dataset;

use oxigraph::MemoryStore;
use oxigraph::store::memory::{MemoryTransaction};
use oxigraph::SledStore;
use oxigraph::store::sled::SledTransaction;
use oxigraph::store::sled::SledTransactionError;
use oxigraph::store::sled::SledConflictableTransactionError;


use std::convert::Infallible;

struct Store<S: Dataset> {
    store: S
}

trait Transact<F, T, R, I, E> 
    where 
        F: Fn(T) -> Result<R, I> {

    fn transaction(&self, f: F) -> Result<R, E>;
}

impl<F, T> Transact<F, SledTransaction<'_>, T, SledConflictableTransactionError<Infallible>, SledTransactionError<Infallible>> for SledStore
    where
        F: Fn(SledTransaction<'_>) -> Result<T, SledConflictableTransactionError<Infallible>> {

    fn transaction(&self, f: F) -> Result<T, SledTransactionError<Infallible>> {
        self.transaction(f)
    }
}

impl<F, E> Transact<F, &mut MemoryTransaction, (), E, E> for MemoryStore
    where
        F: Fn(&mut MemoryTransaction) -> Result<(), E> {

    fn transaction(&self, f: F) -> Result<(), E> {
        self.transaction(f)
    }
}