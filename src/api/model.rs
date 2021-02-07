/// Model
///
/// 

use sophia_api::dataset::Dataset;

use oxigraph::MemoryStore;

use oxigraph::SledStore;
use oxigraph::store::sled::SledTransaction;
use oxigraph::store::sled::SledTransactionError;
use oxigraph::store::sled::SledConflictableTransactionError;

struct Store<S: Dataset> {
    store: S
}

trait Transact<T, I, E> {
    fn transaction(&self, f: impl FnOnce(T) -> Result<(), I>) -> Result<(), E>;
}


impl<'r, E> Transact<SledTransaction<'r>, SledConflictableTransactionError<E>, SledTransactionError<E>> for SledStore {
    fn transaction(&self, f: impl FnOnce(SledTransaction<'r>) -> Result<(), SledConflictableTransactionError<E>>) -> Result<(), SledTransactionError<E>> {
        self.transaction(|transaction: SledTransaction<'r>| {
            f(transaction)
        })
    }
}
