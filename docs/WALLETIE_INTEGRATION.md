# AtDev DB and Walletie

Walletie is an application-level virtual-credit service. Its balance and append-only transaction ledger are owned by Walletie, not by the AtDev DB SQL engine.

## Current architecture

- Walletie V1 stores its wallet tables in the existing AtDev Cloud D1 database.
- AtDev DB remains an independent SQL service optionally used by AtDev Cloud for project data.
- AtDev DB must not be given direct write access to Walletie's wallet or transaction tables.
- AtDev DB does not authenticate Walletie users and must not become a second source of wallet balances.

## Future compatibility

A future AtDev DB adapter may support read-only reporting or analytics through a documented export/API. It must not bypass Walletie's authorization, idempotency, or ledger transaction rules.

The AtDev DB engine remains usable without Walletie. No credits are required to create databases, run SQL, or use core database features.
