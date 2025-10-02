//! Contains trait [`DefaultOp`] used to create a default context.
use crate::{OpSpecId, OpTransaction};
use revm::{
    context::{BlockEnv, CfgEnv, TxEnv, LocalContext},
    database_interface::EmptyDB,
    Context, Journal, MainContext,
};

/// Type alias for the default context type of the ZKsyncEvm.
pub type OpContext<DB> =
    Context<BlockEnv, OpTransaction<TxEnv>, CfgEnv<OpSpecId>, DB, Journal<DB>>;

/// Trait that allows for a default context to be created.
pub trait DefaultOp {
    /// Create a default context.
    fn op() -> OpContext<EmptyDB>;
}

impl DefaultOp for OpContext<EmptyDB> {
    fn op() -> Self {
        Context::mainnet()
            .with_tx(OpTransaction::builder().build_fill())
            .with_cfg(CfgEnv::new_with_spec(OpSpecId::Initial))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::builder::OpBuilder;
    use revm::{
        inspector::{InspectEvm, NoOpInspector},
        ExecuteEvm,
    };

    #[test]
    fn default_run_op() {
        let ctx = Context::op();
        // convert to optimism context
        let mut evm = ctx.build_op_with_inspector(NoOpInspector {});
        // execute
        let _ = evm.transact(OpTransaction::builder().build_fill());
        // inspect
        let _ = evm.inspect_one_tx(OpTransaction::builder().build_fill());
    }
}
