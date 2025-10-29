# Staking UI Service

A scalable backend service for the Espresso delegation UI, designed to support tens of thousands of simultaneous clients with real time data updates.

## Overview

This service provides a polling based REST API that streams blockchain state to UI clients using a **snapshot + updates** pattern. Instead of Web socket, clients poll cache optimized endpoints to receive incremental updates. The state is organized into independent modules sourced from either Ethereum L1 (validator registrations, delegations, withdrawals) or Espresso (validator statistics, rewards). Each module maintains a finalized snapshot in SQL database and unfinalized updates in memory, with automatic handling of L1 reorgs. Clients bootstrap by fetching a complete snapshot indexed by block hash, then poll for sequential updates to maintain realtime state. This design by using the block hashes make most endpoints pure functions that can be cached. This design ensures handling of L1 reorgs, automatic failover between RPC providers, and strong data consistency guarantee across all clients.
