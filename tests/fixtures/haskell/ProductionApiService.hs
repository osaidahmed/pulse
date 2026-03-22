module ProductionApiService where

data PaymentMethod = CreditCard | BankTransfer | Crypto | Wallet

data Currency = USD | EUR | BTC | ETH | GBP

data PaymentResult = PaymentResult
  { txId :: String
  , status :: String
  , message :: String
  }

processPayment :: Double -> Currency -> PaymentMethod -> String -> Either String PaymentResult
processPayment amount currency method userId
  | amount <= 0 = Left "invalid amount"
  | otherwise = case method of
      CreditCard ->
        if amount > 10000
          then Left "credit card limit exceeded"
          else case currency of
            USD -> Right (PaymentResult (ccTxId userId) "pending" "cc-usd")
            EUR -> Right (PaymentResult (ccTxId userId) "pending" "cc-eur")
            _   -> Left "unsupported currency for credit card"
      BankTransfer ->
        if amount < 100
          then Left "minimum amount for bank transfer is 100"
          else Right (PaymentResult (btTxId userId) "pending" "bank")
      Crypto -> case currency of
        BTC -> Right (PaymentResult (crTxId userId) "pending" "btc")
        ETH -> Right (PaymentResult (crTxId userId) "pending" "eth")
        _   -> Left "unsupported crypto currency"
      Wallet -> Right (PaymentResult (wlTxId userId) "pending" "wallet")

ccTxId :: String -> String
ccTxId uid = "cc_" ++ uid

btTxId :: String -> String
btTxId uid = "bt_" ++ uid

crTxId :: String -> String
crTxId uid = "cr_" ++ uid

wlTxId :: String -> String
wlTxId uid = "wl_" ++ uid

getPaymentStatus :: String -> String
getPaymentStatus txid
  | take 3 txid == "cc_" = "credit_card_pending"
  | take 3 txid == "bt_" = "bank_transfer_pending"
  | take 3 txid == "cr_" = "crypto_pending"
  | take 3 txid == "wl_" = "wallet_pending"
  | otherwise = "unknown"

refund :: String -> Either String String
refund txid
  | null txid = Left "empty transaction ID"
  | take 3 txid == "cr_" = Left "crypto refunds not supported"
  | otherwise = Right ("refund_" ++ txid)
