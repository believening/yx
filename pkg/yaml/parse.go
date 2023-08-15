package yaml

import (
	"bytes"
	"io"

	. "github.com/antonmedv/fx/pkg/types"
	"gopkg.in/yaml.v3"
)

type Decoder struct {
	isList *bool
	raw    []byte
	m      map[string]interface{}
	l      []interface{}
}

func NewDecoder(r io.Reader) *Decoder {
	raw, _ := io.ReadAll(r)
	return &Decoder{
		raw: raw,
	}

}

func (d *Decoder) IsList() bool {
	if d.isList != nil {
		return *d.isList
	}
	dec := yaml.NewDecoder(bytes.NewBuffer(d.raw))
	var isList bool
	var m map[string]interface{}
	err := dec.Decode(&m)
	if err != nil {
		isList = true
		var l []interface{}
		_ = dec.Decode(&l)
		d.l = l
	} else {
		d.m = m
	}
	d.isList = &isList
	return *d.isList
}

func Parse(dec *Decoder) (interface{}, error) {
	if dec.IsList() {
		return decodeArray(dec)
	}
	return decodeDict(dec)
}

func dfsDecode(i interface{}) (interface{}, error) {
	switch value := i.(type) {
	case map[string]interface{}:
		innerD := NewDict()
		for k, v := range value {
			innerV, err := dfsDecode(v)
			if err != nil {
				return nil, err
			}
			innerD.Set(k, innerV)
		}
		return innerD, nil
	case []interface{}:
		innerA := make(Array, 0)
		for _, v := range value {
			innerV, err := dfsDecode(v)
			if err != nil {
				return nil, err
			}
			innerA = append(innerA, innerV)
		}
		return innerA, nil
	default:
		return i, nil
	}
}

func decodeDict(dec *Decoder) (*Dict, error) {
	d := NewDict()
	for k, v := range dec.m {
		innerV, err := dfsDecode(v)
		if err != nil {
			return nil, err
		}
		d.Set(k, innerV)
	}
	return d, nil
}

func decodeArray(dec *Decoder) ([]interface{}, error) {
	slice := make(Array, 0)
	for _, v := range dec.l {
		innerV, err := dfsDecode(v)
		if err != nil {
			return nil, err
		}
		slice = append(slice, innerV)
	}
	return slice, nil
}
