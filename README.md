# substrate-feeless-token-factory

The goal of this project is to investigate alternative fee mechanics for managing token assets.

## Background

Ethereum has shown itself to be the ultimate platform for building token economies. Thousands of contracts have been created which support standards like ERC20 and ERC721.

However, businesses using Ethereum have struggled adopting new users into the ecosystem due to the upfront costs of ETH (the native blockchain currency) to interact with these tokens. Many newcomers do not understand why they need ETH to be able to interact with other tokens they actually are interested in.

Businesses have shown that they would be more than happy to fund the usage of their users. Some have done this by providing a faucet or ETH drop to their users, while others may have implemented L2 solutions or have made compromises building centralized solutions.

## What is it?

In simple terms, this project provides a runtime module which provides the following features:

* A token factory where any user is able to create an ERC20 compliant token on top of the Substrate runtime
* An additional API for transfer of these tokens without the end user paying any fees in the native currency

Ideas for alternative payment methods for transfers:

- [x] Token Fee Fund: A fund for a particular token where token transfers are paid from the fund.
- [x] Pay with Token: The ability to "pay" the block producer not with the native currency, but with the token you are trying to transfer.
- [ ] Proof of Work: Complete some small proof of work along with your transfer to allow the transaction to be included.
- [ ] ?

At the time of writing this, the "pay with token" method is not quite supported with our front end libraries.

## User Story

For example, the "Better Energy Foundation" issue a new token to be used as electricity credits.

When they do this, they fund the token with an **initial fund** of the underlying blockchain currency: **10,000 units**. They specify that the users of their token have **10 free transactions every 1,000 blocks**.

They can sell their tokens and transfer them to the buyers just like a normal ICO.

These buyers can then call the `try_free_transfer` function when trying to trade the token among their peers, and the fees are paid for using the fund.

Anyone in the community can continue to add more funds, and allow the free transfers to continue.

If a user does not have any more "free" transactions left for the current period, they can always make a transaction using the normal `transfer` function which will charge them a normal fee.