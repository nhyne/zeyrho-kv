## Chapter 2

So now we have built a _super_ basic queueing server that has a sleep while locking the shared resource. Normally 
this locking would be a bad thing we would want to avoid. However, for our case here it mimics some "complex logic" 
going on inside the server while it is receiving other requests. 

In order to allow requests to continue to be processes without timing out we should implement _some_ feature to 
allow the server to accept a request and store it for later processing. When the server has a cooler period it can 
process those requests and catch up. This is sometimes called [Journaling](localhost:8080). We'll update our 
`Enqueue` endpoint to accept requests and write them to disk to be processes later. The client will receive a 
successful response once the message has been written to disk. We will spawn another process that handles the 
backlog of written-to-disk requests. 

