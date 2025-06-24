## Chapter 3 Replication

Time to try some basic replication with GRPC, it may not be the best method, but we're going to give it a go.

The plan is to stream new messages to the replication client (follower) as they come in. If the client requests a
different offset then we'll have to start from there.

The simple implementation will have problems with replicas starting from random places or from empty DBs. We'll need to
do some sort of "full replica stream" to start from zero. Maybe the first offset sends a whole stream of the current in
memory DB and then proceeds with the next offset from there.