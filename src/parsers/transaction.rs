use core::fmt;

use super::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionType {
    Deposit,
    Transfer,
    Withdrawal,
}

#[derive(Debug, Clone, PartialEq)]
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
    /// Plain text without surrounding quotes. TXT/CSV wrappers are format-level only.
    pub description: String,
}

/// Strip surrounding double quotes and unescape `""` → `"`.
pub fn parse_quoted_description(raw: &str) -> Result<String> {
    let raw = raw.trim();
    if raw.len() < 2 || !raw.starts_with('"') || !raw.ends_with('"') {
        return Err(Error::ParseError(format!(
            "DESCRIPTION must be enclosed in double quotes, got: {}",
            raw
        )));
    }
    let inner = &raw[1..raw.len() - 1];
    Ok(inner.replace("\"\"", "\""))
}

/// Wrap description in double quotes, escaping `"` as `""`.
pub fn format_quoted_description(description: &str) -> String {
    format!("\"{}\"", description.replace('"', "\"\""))
}

impl Transaction {
    pub fn new(
        id: u64,
        operation: TransactionType,
        from_user: u64,
        to_user: u64,
        amount: u64,
        timestamp: u64,
        status: TransactionStatus,
        description: String,
    ) -> Self {
        Transaction {
            id,
            operation,
            from_user,
            to_user,
            amount,
            timestamp,
            status,
            description,
        }
    }

    pub fn diff(&self, other: &Self) -> Option<String> {
        if self.id != other.id {
            return Some(format!("TX_ID: {} vs {}", self.id, other.id));
        }
        if self.operation != other.operation {
            return Some(format!(
                "OPERATION: {} vs {}",
                self.operation, other.operation
            ));
        }
        if self.from_user != other.from_user {
            return Some(format!(
                "FROM_USER: {} vs {}",
                self.from_user, other.from_user
            ));
        }
        if self.to_user != other.to_user {
            return Some(format!("TO_USER_ID: {} vs {}", self.to_user, other.to_user));
        }
        if self.amount != other.amount {
            return Some(format!("AMOUNT: {} vs {}", self.amount, other.amount));
        }
        if self.timestamp != other.timestamp {
            return Some(format!(
                "TIMESTAMP: {} vs {}",
                self.timestamp, other.timestamp
            ));
        }
        if self.status != other.status {
            return Some(format!("STATUS: {} vs {}", self.status, other.status));
        }
        if self.description != other.description {
            return Some(format!(
                "DESCRIPTION: {} vs {}",
                self.description, other.description
            ));
        }

        None
    }
}

impl TransactionType {
    pub fn from_str(s: &str) -> Result<TransactionType> {
        match s {
            "DEPOSIT" => Ok(TransactionType::Deposit),
            "TRANSFER" => Ok(TransactionType::Transfer),
            "WITHDRAWAL" => Ok(TransactionType::Withdrawal),
            _ => Err(Error::ParseError(format!(
                "Unknown transaction type: {}",
                s
            ))),
        }
    }
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

impl TransactionStatus {
    pub fn from_str(s: &str) -> Result<TransactionStatus> {
        match s {
            "FAILURE" => Ok(TransactionStatus::Failure),
            "PENDING" => Ok(TransactionStatus::Pending),
            "SUCCESS" => Ok(TransactionStatus::Success),
            _ => Err(Error::ParseError(format!(
                "Unknown transaction status: {}",
                s
            ))),
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
