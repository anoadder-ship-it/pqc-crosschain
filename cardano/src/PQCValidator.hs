module PQCValidator where

import PlutusTx.Prelude hiding (seq, unless)
import Ledger.Contexts qualified as Ctx
import Ledger.Scripts qualified as Scripts
import Data.ByteString qualified as BS

type PQCDatum = BuiltinByteString -- [height(8)] [nonce(32)] [ct_hash(32)]

validatePQC :: PQCDatum -> BuiltinByteString -> Ctx.ScriptContext -> Bool
validatePQC datum redeemer ctx =
    let txInfo   = Ctx.txInfo ctx
        slot     = Ctx.txInfoSlotRange txInfo
        inputs   = Ctx.txInfoInputs txInfo
        
        height   = decodeWord8 (BuiltinByteStringTail (BuiltinByteStringTake 8 datum))
        nonce    = BuiltinByteStringDrop 8 (BuiltinByteStringTake 40 datum)
        ct_hash  = BuiltinByteStringDrop 40 datum
        
        valid_window = slot >= height && slot <= height + 2
        
        sig_valid    = builtin mkVerifyDilithium redeemer nonce == BuiltinTrue
        ct_bound     = BuiltinSha256 redeemer == ct_hash
        
        no_double_spend = not (any (\i -> Ctx.inputValue i >= totalSupply) inputs)
    in valid_window && sig_valid && ct_bound && no_double_spend

compileValidator :: IO ()
compileValidator = Scripts.compile ''validatePQC