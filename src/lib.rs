
enum TransactionType {
    Deposit,
    Transfer,
    Withdrawal,
}

enum TransactionStatus {
    Success,
    Failure,
    Pending,
}

struct Transaction {
    id: u64,
    operation: TransactionType,
    from_user: u64,
    to_user: u64,
    amount: u64,
    timestamp: u64,
    status: TransactionStatus,
    description: String,
}
