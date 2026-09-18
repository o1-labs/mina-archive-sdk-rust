//! Hand-written GraphQL query strings. Each maps 1:1 to a method on
//! [`ArchiveClient`](crate::ArchiveClient). For narrower selections, send a
//! custom query through [`ArchiveClient::query`](crate::ArchiveClient::query).

pub const EVENTS_QUERY: &str = r#"
query GetEvents($input: EventFilterOptionsInput!) {
  events(input: $input) {
    blockInfo {
      height
      stateHash
      parentHash
      ledgerHash
      chainStatus
      timestamp
      globalSlotSinceHardfork
      globalSlotSinceGenesis
      distanceFromMaxBlockHeight
    }
    eventData {
      accountUpdateId
      transactionInfo {
        status
        hash
        memo
        authorizationKind
        sequenceNumber
        zkappAccountUpdateIds
      }
      data
    }
  }
}
"#;

pub const ACTIONS_QUERY: &str = r#"
query GetActions($input: ActionFilterOptionsInput!) {
  actions(input: $input) {
    blockInfo {
      height
      stateHash
      parentHash
      ledgerHash
      chainStatus
      timestamp
      globalSlotSinceHardfork
      globalSlotSinceGenesis
      distanceFromMaxBlockHeight
    }
    transactionInfo {
      status
      hash
      memo
      authorizationKind
      sequenceNumber
      zkappAccountUpdateIds
    }
    actionData {
      accountUpdateId
      transactionInfo {
        status
        hash
        memo
        authorizationKind
        sequenceNumber
        zkappAccountUpdateIds
      }
      data
    }
    actionState {
      actionStateOne
      actionStateTwo
      actionStateThree
      actionStateFour
      actionStateFive
    }
  }
}
"#;

pub const NETWORK_STATE_QUERY: &str = r#"
query NetworkState {
  networkState {
    maxBlockHeight {
      canonicalMaxBlockHeight
      pendingMaxBlockHeight
    }
  }
}
"#;

pub const BLOCKS_QUERY: &str = r#"
query GetBlocks($query: BlockQueryInput, $limit: Int, $sortBy: BlockSortByInput) {
  blocks(query: $query, limit: $limit, sortBy: $sortBy) {
    blockHeight
    creator
    stateHash
    parentHash
    dateTime
    transactions {
      coinbase
      userCommands {
        hash
        kind
        from
        to
        amount
        fee
        memo
        nonce
        status
        failureReason
      }
      zkappCommands {
        hash
        feePayer
        fee
        memo
        status
        failureReason
      }
      feeTransfer {
        recipient
        fee
        type
      }
    }
  }
}
"#;

pub const VERIFICATION_KEY_UPDATES_QUERY: &str = r#"
query GetVerificationKeyUpdates($input: VerificationKeyUpdateFilterInput!) {
  verificationKeyUpdates(input: $input) {
    accountUpdateId
    address
    tokenId
    verificationKeyHash
    blockInfo {
      height
      stateHash
      parentHash
      ledgerHash
      chainStatus
      timestamp
      globalSlotSinceHardfork
      globalSlotSinceGenesis
      distanceFromMaxBlockHeight
    }
    transactionInfo {
      status
      hash
      memo
      authorizationKind
      sequenceNumber
      zkappAccountUpdateIds
    }
  }
}
"#;
