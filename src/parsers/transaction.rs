use core::fmt;

#[derive(Debug, Clone)]
pub enum TransactionType {
    Deposit,
    Transfer,
    Withdrawal,
}

#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Success,
    Failure,
    Pending,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: u64,
    pub operation: TransactionType,
    pub from_user: u64,
    pub to_user: u64,
    pub amount: u64,
    pub timestamp: u64,
    pub status: TransactionStatus,
    pub description: String,
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionType::Deposit => write!(f, "DEPOSIT"),
            TransactionType::Transfer => write!(f, "TRANSFER"),
            TransactionType::Withdrawal => write!(f, "WITHDRAWAL"),
        }
    }
}

impl fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionStatus::Failure => write!(f, "FAILURE"),
            TransactionStatus::Pending => write!(f, "PENDING"),
            TransactionStatus::Success => write!(f, "SUCCESS"),
        }
    }
}
