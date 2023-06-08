
# Examples of interacting with a InfraBlockSpace node

Some examples of using JSON RPC to interact with a InfraBlockSpace node, working up to manually building and submitting a balance transfer.

To run these examples, first start up a local InfraBlockSpace node (which we'll be interacting with):

Once you have this node running, in another terminal, pick an example you'd like to run from the `src/bin` folder and run it like so:

```
// transfer balance
cargo run --bin 05_transfer_balance
// transfer xcm
cargo run --bin 08_xcm
```
