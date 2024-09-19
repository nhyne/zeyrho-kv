## Zeyrho Queue

The long term plan of this project is to accomplish 3 things:

1. Learn (some) Rust, hopefully a lot
2. Learn distributed systems by building a basic distributed queue that has things like failover, replication, etc...
3. Teach other people distributed systems

Those are some pretty lofty goals, breaking them down into smaller parts:

1. Simple queue that can be used in a basic rust program. Nothing fancy at all, no forking or anything.
2. Make that queue memory safe across processes -- this is not distributed "systems" but still has some interesting learning / teaching points IRT memory safety, locks, concurrent writes, etc...
3. Turn the memory safe queue into a grpc (or http) server
4. Build replication into the server, i.e. a secondary server can be spun up and "follow" a leader
5. Build "new node coming online" logic -- catching up so to speak and then streaming data
6. Build journaling support for when too much is being written
6. Build in failover
   1. This probably will need to be broken down into smaller parts
   2. Do we want a control plane? This is where we'll dive into more DS topics and look at tradeoffs
7. Other? There are a lot of stretch goals
   1. Concurrent writes
   2. Partitioning
   3. 

### Nix setup for intellij

The toolchain path is where cargo is listed after running `which -a cargo`. You have to pick the correct version based on the toolchain for the project.

The std-lib needs to be downloaded with `rustup -v component add rust-src` and then lives in the same path as cargo like `/home/nhyne/.rustup/toolchains/nightly-2024-09-17-x86_64-unknown-linux-gnu/lib/rustlib/src/rust`.
