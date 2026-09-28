package infernalcbor

import (
	"path/filepath"
	"testing"
)

func TestGeneratedFixture(t *testing.T) {
	monster, err := Load(filepath.Join("..", "fixtures", "monster.cbor"))
	if err != nil {
		t.Fatal(err)
	}
	if monster.ID == "" || len(monster.Animations) == 0 {
		t.Fatalf("incomplete monster: %+v", monster)
	}
	if monster.Gameplay == nil || monster.Gameplay.Health == 0 || monster.Generation == nil || monster.Generation.Seed != 42 {
		t.Fatalf("nested metadata missing: %+v", monster)
	}
	hasSpawn := false
	for _, attack := range monster.Attacks {
		hasSpawn = hasSpawn || attack.Spawn != nil
	}
	if len(monster.Projectiles) == 0 || !hasSpawn {
		t.Fatal("projectile or spawn metadata missing")
	}
}
