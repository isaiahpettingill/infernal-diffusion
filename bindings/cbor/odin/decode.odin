package infernal_cbor

import "core:encoding/cbor"

decode_monster :: proc(data: []u8) -> (monster: Monster, err: cbor.Unmarshal_Error) {
    err = cbor.unmarshal(data, &monster)
    return
}
