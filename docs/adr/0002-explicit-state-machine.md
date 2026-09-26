# ADR 0002: Explicit execution state machine
Status: Accepted

CER uses explicit states rather than independent booleans. `EXECUTION_UNKNOWN` is first-class because a lost response after a remote effect cannot safely be represented as ordinary failure. Illegal transitions are rejected by the kernel and tested.
