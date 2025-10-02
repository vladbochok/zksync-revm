//! Optimism builder trait [`OpBuilder`] used to build [`ZKsyncEvm`].
use crate::{evm::ZKsyncEvm, precompiles::OpPrecompiles, transaction::OpTxTr, L1BlockInfo, OpSpecId};
use revm::{
    context::Cfg,
    context_interface::{Block, JournalTr},
    handler::instructions::EthInstructions,
    interpreter::interpreter::EthInterpreter,
    state::EvmState,
    Context, Database,
};

/// Type alias for default ZKsyncEvm
pub type DefaultZKsyncEvm<CTX, INSP = ()> =
    ZKsyncEvm<CTX, INSP, EthInstructions<EthInterpreter, CTX>, OpPrecompiles>;

/// Trait that allows for optimism ZKsyncEvm to be built.
pub trait OpBuilder: Sized {
    /// Type of the context.
    type Context;

    /// Build the op.
    fn build_op(self) -> DefaultZKsyncEvm<Self::Context>;

    /// Build the op with an inspector.
    fn build_op_with_inspector<INSP>(self, inspector: INSP) -> DefaultZKsyncEvm<Self::Context, INSP>;
}

impl<BLOCK, TX, CFG, DB, JOURNAL> OpBuilder for Context<BLOCK, TX, CFG, DB, JOURNAL, L1BlockInfo>
where
    BLOCK: Block,
    TX: OpTxTr,
    CFG: Cfg<Spec = OpSpecId>,
    DB: Database,
    JOURNAL: JournalTr<Database = DB, State = EvmState>,
{
    type Context = Self;

    fn build_op(self) -> DefaultZKsyncEvm<Self::Context> {
        ZKsyncEvm::new(self, ())
    }

    fn build_op_with_inspector<INSP>(self, inspector: INSP) -> DefaultZKsyncEvm<Self::Context, INSP> {
        ZKsyncEvm::new(self, inspector)
    }
}
