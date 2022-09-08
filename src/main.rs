use csv::StringRecord;

extern crate csv;

const AMOUNT_PRECISION_LIMITER: u16 = 10000;

enum TransactionType {
    Deposit,
    Withdraw,
    Dispute,
    Resolve,
    Chargeback,
    Invalid,
}

impl From<&str> for TransactionType {
    fn from(value: &str) -> Self {
        match value {
            "deposit" => TransactionType::Deposit,
            "withdrawal" => TransactionType::Withdraw,
            "dispute" => TransactionType::Dispute,
            "resolve" => TransactionType::Resolve,
            "chargeback" => TransactionType::Chargeback,
            _ => TransactionType::Invalid,
        }
    }
}

#[derive(Clone, Copy)]
struct Amount {
    whole: i64,
    decimal: u16,
}

impl core::cmp::PartialEq for Amount {
    fn eq(&self, other: &Self) -> bool {
        (self.whole == other.whole) && (self.decimal == other.decimal)
    }

    fn ne(&self, other: &Self) -> bool {
        (self.whole != other.whole) || (self.decimal != other.decimal)
    }
}

impl core::cmp::PartialOrd for Amount {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            return Some(std::cmp::Ordering::Equal);
        } else if self < other {
            return Some(std::cmp::Ordering::Less);
        } else {
            return Some(std::cmp::Ordering::Greater);
        }
    }

    fn ge(&self, other: &Self) -> bool {
        self.eq(other)
            || (self.whole > other.whole)
            || ((self.whole >= other.whole) && (self.decimal >= other.decimal))
    }

    fn gt(&self, other: &Self) -> bool {
        (self.whole > other.whole)
            || ((self.whole == other.whole) && (self.decimal > other.decimal))
    }

    fn le(&self, other: &Self) -> bool {
        self.eq(other)
            || (self.whole < other.whole)
            || ((self.whole <= other.whole) && (self.decimal <= other.decimal))
    }

    fn lt(&self, other: &Self) -> bool {
        (self.whole < other.whole)
            || ((self.whole == other.whole) && (self.decimal < other.decimal))
    }
}

impl std::ops::Add for Amount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut w_add_res = self.whole + rhs.whole;
        let mut d_add_res = self.decimal + rhs.decimal;
        if d_add_res >= AMOUNT_PRECISION_LIMITER {
            w_add_res += (d_add_res / AMOUNT_PRECISION_LIMITER) as i64;
            d_add_res %= AMOUNT_PRECISION_LIMITER;
        }
        Amount {
            whole: w_add_res,
            decimal: d_add_res,
        }
    }
}

impl std::ops::Sub for Amount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut w_sub_res = self.whole - rhs.whole;
        let d_sub_res;
        if rhs.decimal > self.decimal {
            w_sub_res -= 1;
            d_sub_res = rhs.decimal - self.decimal;
        } else {
            d_sub_res = self.decimal - rhs.decimal;
        }
        Amount {
            whole: w_sub_res,
            decimal: d_sub_res,
        }
    }
}

impl From<&str> for Amount {
    fn from(value: &str) -> Self {
        if value.contains(".") {
            let splits = value.split(".").collect::<Vec<_>>();
            let w = splits[0].parse::<i64>().unwrap_or(0);
            let mut d = splits[1].parse::<u16>().unwrap_or(0);
            while d >= AMOUNT_PRECISION_LIMITER {
                d = d / 10;
            }
            return Amount {
                whole: w,
                decimal: d,
            };
        } else {
            return Amount {
                whole: value.parse::<i64>().unwrap_or(0),
                decimal: 0,
            };
        }
    }
}

impl From<i64> for Amount {
    fn from(value: i64) -> Self {
        Amount {
            whole: value,
            decimal: 0,
        }
    }
}

impl Default for Amount {
    fn default() -> Self {
        Amount {
            whole: 0,
            decimal: 0,
        }
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.whole, self.decimal)
    }
}

struct Transaction {
    tr_type: TransactionType,
    client_id: u16,
    tr_id: u32,
    amount: Option<Amount>,
}

impl From<StringRecord> for Transaction {
    fn from(rec: StringRecord) -> Self {
        Transaction {
            tr_type: TransactionType::from(rec.get(0).expect("Invalid Transaction")),
            client_id: rec
                .get(1)
                .expect("Client ID not found")
                .parse::<u16>()
                .unwrap_or(0),
            tr_id: rec
                .get(2)
                .expect("Transaction ID not found")
                .parse::<u32>()
                .unwrap_or(0),
            amount: if rec.len() == 4 {
                Some(Amount::from(rec.get(3).expect("Amount not found")))
            } else {
                None
            },
        }
    }
}

struct AccountStatus {
    client_id: u16,
    available: Amount,
    held: Amount,
    locked: bool,
}

impl AccountStatus {
    fn total_amount(&self) -> Amount {
        self.available + self.held
    }
}

impl std::fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},        {},     {},   {},  {}",
            self.client_id,
            self.available,
            self.held,
            self.total_amount(),
            self.locked
        )
    }
}

fn handle_account(id: u16, statuses: &Vec<AccountStatus>) -> Option<usize> {
    for (i, r) in statuses.iter().enumerate() {
        if r.client_id == id {
            return Some(i);
        }
    }
    None
}

fn get_transaction_with_id<'a>(
    tr_id: u32,
    transactions: &'a Vec<Transaction>,
) -> Option<&'a Transaction> {
    for tr in transactions {
        if tr.tr_id == tr_id {
            return Some(tr);
        }
    }
    None
}

fn is_disputed_transaction(id: u32, dis: &Vec<u32>) -> bool {
    dis.iter().position(|&el| -> bool { el == id }).is_some()
}

fn remove_dispute(id: u32, dis: &mut Vec<u32>) {
    dis.retain(|&e| e != id);
}

fn process_transactions<'a>(trs: &'a mut Vec<Transaction>) -> Vec<AccountStatus> {
    let mut result: Vec<AccountStatus> = vec![];
    let mut disputes: Vec<u32> = vec![];
    for (_i, tr) in trs.iter().enumerate() {
        let index = handle_account(tr.client_id, &result).unwrap_or(result.len());
        if index == result.len() {
            result.push(AccountStatus {
                client_id: tr.client_id,
                available: Amount::default(),
                held: Amount::default(),
                locked: false,
            });
        }
        let el = result.get_mut(index).expect("No account status found");
        match tr.tr_type {
            TransactionType::Deposit => {
                if !el.locked {
                    el.available = el.available + tr.amount.expect("No amount found for deposit");
                }
            }
            TransactionType::Withdraw => {
                if !el.locked {
                    if (el.available - tr.amount.expect("No amount found for withdrawal"))
                        >= Amount::default()
                    {
                        el.available =
                            el.available - tr.amount.expect("No amount found for withdrawal");
                    }
                }
            }
            TransactionType::Dispute => {
                if !el.locked {
                    let candidate_tr = get_transaction_with_id(tr.tr_id, trs);
                    if candidate_tr.is_some() {
                        let c_tr = candidate_tr.expect("");
                        disputes.push(c_tr.tr_id);
                        let candidate_amount = c_tr.amount.expect("No amount found for dispute");
                        el.available = el.available - candidate_amount;
                        el.held = el.held + candidate_amount;
                    }
                }
            }
            TransactionType::Resolve => {
                if !el.locked {
                    let candidate_tr = get_transaction_with_id(tr.tr_id, trs);
                    if candidate_tr.is_some() {
                        let c_tr = candidate_tr.expect("");
                        if is_disputed_transaction(c_tr.tr_id, &disputes) {
                            let candidate_amount =
                                c_tr.amount.expect("No amount found for resolve");
                            el.available = el.available + candidate_amount;
                            el.held = el.held - candidate_amount;
                            remove_dispute(c_tr.tr_id, &mut disputes);
                        }
                    }
                }
            }
            TransactionType::Chargeback => {
                if !el.locked {
                    let candidate_tr = get_transaction_with_id(tr.tr_id, trs);
                    if candidate_tr.is_some() {
                        let c_tr = candidate_tr.expect("");
                        if is_disputed_transaction(c_tr.tr_id, &disputes) {
                            let candidate_amount =
                                c_tr.amount.expect("No amount found for chargeback");
                            el.held = el.held - candidate_amount;
                            el.locked = true;
                            remove_dispute(c_tr.tr_id, &mut disputes);
                        }
                    }
                }
            }
            TransactionType::Invalid => {
                eprintln!("Invalid transaction found")
            }
        }
    }
    result
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 {
        let mut transactions: Vec<Transaction> = vec![];
        let csv_reader = csv::Reader::from_path(args[1].as_str());
        match csv_reader {
            Ok(mut reader) => {
                for result in reader.records() {
                    if result.is_ok() {
                        transactions.push(Transaction::from(result.unwrap()));
                    }
                }
                let account_statuses = process_transactions(&mut transactions);
                println!("Client, Available, Held, Total, Locked");
                for account in account_statuses {
                    println!("{}", account);
                }
            }
            Err(_) => eprintln!("Could not create CSV reader for path: {}", args[1]),
        }
    } else {
        eprintln!("No path for the CSV file provided");
    }
}
