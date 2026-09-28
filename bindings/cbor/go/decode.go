package infernalcbor

import (
	"fmt"
	"os"

	"github.com/fxamacker/cbor/v2"
)

func Decode(data []byte) (Monster, error) {
	var monster Monster
	if err := cbor.Unmarshal(data, &monster); err != nil {
		return Monster{}, err
	}
	if monster.FormatVersion < 2 || monster.FormatVersion > 7 {
		return Monster{}, fmt.Errorf("unsupported monster format_version %d", monster.FormatVersion)
	}
	return monster, nil
}

func Load(path string) (Monster, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return Monster{}, err
	}
	return Decode(data)
}
