package yaml

import (
	"fmt"

	. "github.com/antonmedv/fx/pkg/types"
)

func Stringify(v interface{}) string {
	switch v := v.(type) {
	case nil:
		return "null"

	case bool:
		if v {
			return "true"
		} else {
			return "false"
		}

	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64, float32, float64:
		return fmt.Sprintf("%v", v)

	case string:
		return fmt.Sprintf("%q", v)

	case *Dict:
		result := "{"
		for i, key := range v.Keys {
			line := fmt.Sprintf("%q", key) + ": " + Stringify(v.Values[key])
			if i < len(v.Keys)-1 {
				line += ","
			}
			result += line
		}
		return result + "}"

	case Array:
		result := "["
		for i, value := range v {
			line := Stringify(value)
			if i < len(v)-1 {
				line += ","
			}
			result += line
		}
		return result + "]"

	default:
		return "unknown type"
	}
}
