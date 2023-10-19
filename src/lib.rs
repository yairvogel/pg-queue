use postgres::{Client, NoTls};

pub struct QueueClient {
    pClient: Client
}

impl QueueClient {
    pub fn connect(connection_string: &str) -> Self {
        QueueClient {
            pClient: Client::connect(connection_string, NoTls).unwrap()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let qc = QueueClient::connect(&"host=localhost user=postgres db=db");

    }
}
