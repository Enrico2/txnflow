# TxnFlow

## Rationale and Design Choices

### Async

// TODO(ran) FIXME: write

### The Store traits

// TODO(ran) FIXME: write

### StoredTxn vs. Storing the full row.

// TODO(ran) FIXME: write

## Potential Optimizations for Production

// TODO(ran) FIXME: write

### Store all transactions

// TODO(ran) FIXME: write

### Shard by customer id to maintain ordering for parallel stream computation

// TODO(ran) FIXME: write

## Interpretation and assumptions

Going through the various use cases, I found either contradictions or lack of reason in the instructions. It's as if the instructions only consider a user
disputing a `deposit` transaction and not a `withdrawal` transaction:

### The instructions:

1. `dispute`: `available -= amount_disputed`; `held += amount_disputed` ("_the clients available funds should decrease by the amount disputed, their held funds
   should increase by the amount disputed_")
1. `resolve`: `available += amount_disputed`; `held -= amount_disputed` ("_the clients held funds should decrease by the amount no longer disputed, their
   available funds should increase by the amount no longer disputed_")
1. `chargeback`: `held -= amount_disputed` ("_the clients held funds and **total** funds should decrease by the amount previously disputed_")
1. `chargeback` represents the client **reversing** a transaction

### Use cases:

The interpretation here is that a `resolve` restores the account to the state it was in prior to the dispute (i.e. reversing the change caused by the `dispute`)
, and that a `chargeback` restores the account to the state it was in prior to the transaction under dispute (i.e. reversing the transaction).

Numbers below are in the format of `available,held`, withdrawal and deposits are of $100.

```
start: 0,0
deposit => 100,0
dispute => 0,100
resolve => 100,0

start: 0,0
deposit    => 100,0
dispute    => 0,100
chargeback => 0,0
```

clearly, when a `deposit` is disputed, following the instructions works and makes sense.

However, when it comes to a `withdrawal`:

--- following instructions -----

```
start: 100,0
withdrawal => 0,0
dispute    => -100,100 : Doesn't make sense
resolve    => 0,0

start: 100,0
withdrawal => 0,0   
dispute    => -100,100 : Doesn't make sense
chargeback => -100,0   : Contradicts (4) and doesn't make sense

```

#### A note on negative numbers

I considered treating the "disputed amount" as a negative number in case of a withdrawal, however, that would result in the held amount potentially being
negative.

My understanding is that a `held` amount means the bank saying "Maybe you have these funds, maybe you don't, for now we're holding onto them." which implies it
would be positive both in case of a deposit or withdrawal.

--- following instructions but the disputed amount is negative -----

```
start: 100,0
withdrawal => 0,0
dispute    => 100,-100 : Doesn't make sense. There should be no available amount because it's under dispute.
...

```

### My interpretation

--- Logical way to handle a withdrawal dispute -----

```
start: 100,0
withdrawal => 0,0
dispute    => 0,100 : Holds the disputed amount to potentially return to the user, but contradicts (1).
resolve    => 0,0   : Releases back the held amount to the withdrawal destination, but contradicts (2).

start: 100,0
withdrawal => 0,0   
dispute    => 0,100 : Holds the disputed amount to potentially return to the user, but contradicts (1).
chargeback => 100,0 : Reverses the transaction, but contradicts (3).
```

--------

### Other Assumptions:

1. A transaction on a `locked` account is ignored and logged to stderr.
1. A transaction can be disputed more than once if it was previously resolved (account is not locked)
1. A dispute can result in an account going into negative balance.
1. Transaction ids are unique (as described in the instructions)
1. The system supports deposits & withdrawal of an amount equal to 0. I figured there might be a case for disputing those as well and then I'd need them stored.

