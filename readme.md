## CSV Payment Processor

- Run using cargo: `cargo run -- path_to_file.csv`
- CSV Format expected:
  | Type | Client | Transaction | Amount |
  | :-: | :-: | :-: | :-: |
  | deposit | 1 | 1 | 1.0 |
  | deposit | 2 | 2 | 2.0 |
  | deposit | 1 | 3 | 2.0 |
  | withdrawal | 1 | 4 | 1.5 |
  | withdrawal | 2 | 5 | 2.0 |

