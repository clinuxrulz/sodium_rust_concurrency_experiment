# Sodium
A Functional Reactive Programming (FRP) library for Rust

## Pitfalls

### No Global State

You must create a SodiumCtx for your application and keep passing it around in order to create sodium objects.
