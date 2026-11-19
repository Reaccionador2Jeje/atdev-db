# AtDev Billing — Hybrid plans (AtDev DB)

**Status:** proposal only. AtDev DB is experimental alpha software; no production database hosting or paid service is being offered.

## Free software, optional managed service

The SQL engine, source code, local use, and self-hosting remain free. A future AtDev-operated managed database service may offer convenience features only after the engine meets production reliability, security, backup, and recovery requirements.

## Proposed future service tiers (CLP/month; exploratory)

| Plan | Reference price | Scope |
| --- | ---: | --- |
| Local / Self-hosted | $0 | Run the open-source engine on infrastructure you control |
| Managed Sandbox | $990 | Disposable development database with strict storage and request quotas |
| Managed Developer | $1,990 | More development databases, automated backups, and usage metrics |

Do not market these as production-ready while the engine remains experimental. Prices must be recalculated from compute, storage, backups, bandwidth, support, fees, and taxes.

## Entitlements

Suggested keys: `db.managed.instances`, `db.managed.storage_bytes`, `db.managed.requests.monthly`, `db.managed.backups`, `db.managed.retention_days`.

Enforce limits server-side. Do not grant access based on client-side plan names. Billing outages must not corrupt data; document grace periods, read-only fallback, export, restore, and deletion behavior before launch.

## Launch gates

Require production-grade durability tests, backup/restore drills, tenant isolation, security review, metering, and an adult account owner's review of payment-provider eligibility and legal/tax obligations. Checkout remains disabled.
