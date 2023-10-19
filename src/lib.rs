use uuid::Uuid;
use std::time::SystemTime;

pub struct QueueClient {
    p_client: postgres::Client,
    queue_name: String,
}

pub struct QueueMessage {
    pub id: Uuid,
    pub inserted_at: SystemTime,
    pub data: Vec<u8>
}

#[derive(Debug)]
pub enum Error {
    PostgresError(postgres::Error),
    QueueError(&'static str),
}

impl QueueClient {
    pub fn connect(connection_string: &str, queue_name: &str) -> Result<Self, postgres::Error> {
        let p_client = postgres::Client::connect(connection_string, postgres::NoTls)?;
        Ok(Self::use_db_client(p_client, queue_name))
    }

    pub fn use_db_client(client: postgres::Client, queue_name: &str) -> Self {
        QueueClient {
            p_client: client,
            queue_name: queue_name.to_owned()
        }
    }

    pub fn enqueue(&mut self, data: &[u8]) -> Result<(), Error> {
        let result = self.p_client.execute(format!("INSERT INTO {table} (data) VALUES ($1)", table=self.queue_name).as_str(), &[&data]);
        match result {
            Err(err) => Err(Error::PostgresError(err)),
            _ => Ok(())
        }
    }

    pub fn dequeue(&mut self) -> Result<Option<QueueMessage>, Error> {
        let response = self.p_client.query(format!("
            DELETE FROM {table} q
            WHERE q.id = (
                 SELECT q_in.id
                 FROM {table} q_in
                 ORDER BY q_in.inserted_at ASC
                 FOR UPDATE SKIP LOCKED
                 LIMIT 1)
            RETURNING q.id, q.inserted_at, q.data", table=self.queue_name).as_str(), &[]);

        let result = match response {
            Err(err) => return Err(Error::PostgresError(err)),
            Ok(result) => result
        };


        let row = match result.len() {
            1 => &result[0],
            0 => return Ok(None),
            _ => return Err(Error::QueueError("expected one row, got multiple rows."))
        };

        Ok(Some(QueueMessage {
            id: row.get::<usize, Uuid>(0),
            inserted_at: row.get::<usize, SystemTime>(1),
            data: row.get::<usize, Vec<u8>>(2)
        }))
    }

    pub fn try_create_queue(&mut self) -> Result<(), postgres::Error> {
        self.p_client.batch_execute(
            format!("CREATE TABLE IF NOT EXISTS {table} (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
                data BYTEA);
            CREATE INDEX IF NOT EXISTS inserted_at ON {table} (inserted_at);", table=&self.queue_name).as_str())?;

        Ok(())
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    const DB_CONNSTRING: &str = "host=localhost user=postgres dbname=db";

    #[test]
    fn send_and_receive() {
        let mut qc = QueueClient::connect(DB_CONNSTRING, "queue").unwrap();
        qc.try_create_queue().unwrap();
        qc.enqueue("Hello World".as_bytes()).expect("should be able to insert message");
        assert_eq!("queue", qc.queue_name.as_str());
        let message = qc.dequeue().unwrap();
        assert_eq!(message.expect("should contain a row").data, "Hello World".as_bytes());
    }

    #[test]
    fn receive_without_message() {
        let mut qc = QueueClient::connect(DB_CONNSTRING, "queue").unwrap();
        qc.try_create_queue().unwrap();
        let message = qc.dequeue().expect("should be able to query for message");
        assert!(message.is_none());
    }

    #[test]
    fn receive_ordered() {
        let mut pclient = QueueClient::connect(DB_CONNSTRING, "queue_ordered").unwrap();
        pclient.try_create_queue().expect("should be able to create queue"); 

        for n in 1i32..10 {
            pclient.enqueue(&n.to_ne_bytes()).expect("should be able to enqueue");
        }

        for expected in 1i32..10 {
            let message = pclient.dequeue().unwrap().unwrap();
            let actual = deserialize_i32(&message.data);
            assert_eq!(expected, actual)
        }
    }

    fn deserialize_i32(bytes: &[u8]) -> i32 {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(bytes);

        i32::from_ne_bytes(buf)
    }
}
