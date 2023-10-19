![tests](https://github.com/yairvogel/pg-queue/actions/workflows/tests.yml/badge.svg)

# pg-queue
Rust implementation for a FIFO queue, based on postgresql


## Usage
Create a QueueClient with a valid postgres connection string and a queue name.

QueueClient contains enqueue and dequeue functionality, and guarantees a First in-First out behaviour.
