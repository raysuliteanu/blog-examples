# Simple HTTP Server in Rust
This code started from the [CodeCrafters](https://codecrafters.io) "Build your own HTTP server"
challenge. This challenge evolved in several steps, to support the following
* a simple 'echo' endpoint
* parsing and returning a header value
* downloading a file
* uploading a file

As such at this stage it's kind of a "strange" HTTP server (and with this commit doesn't
even implement the file upload use case. But I wanted to commit the code to my own repo
as I evolve it and use it in any blogs I might write.

##TODO
There are a lot of things I could still do. For one thing, the current parsing "cheats"
in that for a file upload, I don't actually look at the `Content-Length` header. The `nom`
-based parser reads until there's nothing to read, and whatever isn't part of the request block
is part of the body. This is obviously not idea for large files.

