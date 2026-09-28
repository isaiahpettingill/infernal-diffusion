package main

import infernal "../odin"
import "core:fmt"
import "core:os"

main :: proc() {
    data, file_err := os.read_entire_file("bindings/cbor/fixtures/monster.cbor", context.allocator)
    fmt.assertf(file_err == nil, "could not read CBOR fixture: %v", file_err)
    defer delete(data)
    monster, decode_err := infernal.decode_monster(data)
    fmt.assertf(decode_err == nil, "could not decode CBOR monster: %v", decode_err)
    fmt.assertf(monster.id != "" && len(monster.animations) > 0, "incomplete CBOR monster")
    fmt.assertf(monster.generation != nil && monster.generation.?.seed == 42, "nested CBOR metadata missing")
    has_spawn := false
    for attack in monster.attacks {
        has_spawn = has_spawn || attack.spawn != nil
    }
    fmt.assertf(len(monster.projectiles) > 0 && has_spawn, "projectile or spawn metadata missing")
    fmt.println(monster.id, monster.display_name)
}
