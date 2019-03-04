# Current

- get toolchains
- format progressbar style from them
- find target dir
- set up progress bars and multi bar
- join multi
- get worker count based on checkouts, job, cli, and num of cpus
- spawn workers
- do checkouts
  - send checkout to worker pool once done
- wait on all workers and check success
- join multi

# Ideal

- get toolchains
- find target dir

- optionally spawn workers
  - empty worker handle vec is still foldable with true default
- make closure for reporting a finished checkout
  
- do checkouts
  - send checkout to opaque handler type

- wait on all (perhaps there aren't any) workers

- join opaque progress report type