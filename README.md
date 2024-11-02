# JetBrains Blog Task

Simple web application simulating very simple online message board.

The application is written in Rust, with SQLite as a database storage.

### Standalone usage

The application takes one optional argument, that being the location of the SQLite file. The usage may look like this:

```shell
cargo run --release -- --db-file blog.db
```

By default, the server runs on HTTP port 3000, and the main page is at `/home`, i.e. you can find it by opening `localhost:3000/home` in your browser. 

### Usage in Docker

The application could be run in Docker using the following command:

```shell
sudo docker compose up
```

The data will be saved to `docker_data/blog.db`.

Unfortunately, the application doesn't appear to run in Docker, or at least not in a manner that I would personally consider satisfactory - downloading of avatar images does not seem to work. It could be incorrect setup of my machine, and I could be wrong about any part of this, but the Docker container as a whole appears to be unable to make requests over IPv6 (I tested this with Curl), and it appears as though the `reqwest` library prefers making HTTP requests over IPv6, even when IPv4 address is set as the `local_address`.
