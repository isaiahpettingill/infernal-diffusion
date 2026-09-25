#
# BSD 3-Clause License
#
# Copyright (c) 2018 - 2026, Oleg Malyavkin
# All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions are met:
#
# * Redistributions of source code must retain the above copyright notice, this
#   list of conditions and the following disclaimer.
#
# * Redistributions in binary form must reproduce the above copyright notice,
#   this list of conditions and the following disclaimer in the documentation
#   and/or other materials provided with the distribution.
#
# * Neither the name of the copyright holder nor the names of its
#   contributors may be used to endorse or promote products derived from
#   this software without specific prior written permission.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
# AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
# IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
# DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
# FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
# DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
# SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
# CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
# OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

# DEBUG_TAB redefine this "  " if you need, example: const DEBUG_TAB = "\t"

const PROTO_VERSION = 3

const DEBUG_TAB : String = "  "

enum PB_ERR {
	NO_ERRORS = 0,
	VARINT_NOT_FOUND = -1,
	REPEATED_COUNT_NOT_FOUND = -2,
	REPEATED_COUNT_MISMATCH = -3,
	LENGTHDEL_SIZE_NOT_FOUND = -4,
	LENGTHDEL_SIZE_MISMATCH = -5,
	PACKAGE_SIZE_MISMATCH = -6,
	UNDEFINED_STATE = -7,
	PARSE_INCOMPLETE = -8,
	REQUIRED_FIELDS = -9
}

enum PB_DATA_TYPE {
	INT32 = 0,
	SINT32 = 1,
	UINT32 = 2,
	INT64 = 3,
	SINT64 = 4,
	UINT64 = 5,
	BOOL = 6,
	ENUM = 7,
	FIXED32 = 8,
	SFIXED32 = 9,
	FLOAT = 10,
	FIXED64 = 11,
	SFIXED64 = 12,
	DOUBLE = 13,
	STRING = 14,
	BYTES = 15,
	MESSAGE = 16,
	MAP = 17
}

const DEFAULT_VALUES_2 = {
	PB_DATA_TYPE.INT32: null,
	PB_DATA_TYPE.SINT32: null,
	PB_DATA_TYPE.UINT32: null,
	PB_DATA_TYPE.INT64: null,
	PB_DATA_TYPE.SINT64: null,
	PB_DATA_TYPE.UINT64: null,
	PB_DATA_TYPE.BOOL: null,
	PB_DATA_TYPE.ENUM: null,
	PB_DATA_TYPE.FIXED32: null,
	PB_DATA_TYPE.SFIXED32: null,
	PB_DATA_TYPE.FLOAT: null,
	PB_DATA_TYPE.FIXED64: null,
	PB_DATA_TYPE.SFIXED64: null,
	PB_DATA_TYPE.DOUBLE: null,
	PB_DATA_TYPE.STRING: null,
	PB_DATA_TYPE.BYTES: null,
	PB_DATA_TYPE.MESSAGE: null,
	PB_DATA_TYPE.MAP: null
}

const DEFAULT_VALUES_3 = {
	PB_DATA_TYPE.INT32: 0,
	PB_DATA_TYPE.SINT32: 0,
	PB_DATA_TYPE.UINT32: 0,
	PB_DATA_TYPE.INT64: 0,
	PB_DATA_TYPE.SINT64: 0,
	PB_DATA_TYPE.UINT64: 0,
	PB_DATA_TYPE.BOOL: false,
	PB_DATA_TYPE.ENUM: 0,
	PB_DATA_TYPE.FIXED32: 0,
	PB_DATA_TYPE.SFIXED32: 0,
	PB_DATA_TYPE.FLOAT: 0.0,
	PB_DATA_TYPE.FIXED64: 0,
	PB_DATA_TYPE.SFIXED64: 0,
	PB_DATA_TYPE.DOUBLE: 0.0,
	PB_DATA_TYPE.STRING: "",
	PB_DATA_TYPE.BYTES: [],
	PB_DATA_TYPE.MESSAGE: null,
	PB_DATA_TYPE.MAP: []
}

enum PB_TYPE {
	VARINT = 0,
	FIX64 = 1,
	LENGTHDEL = 2,
	STARTGROUP = 3,
	ENDGROUP = 4,
	FIX32 = 5,
	UNDEFINED = 8
}

enum PB_RULE {
	OPTIONAL = 0,
	REQUIRED = 1,
	REPEATED = 2,
	RESERVED = 3
}

enum PB_SERVICE_STATE {
	FILLED = 0,
	UNFILLED = 1
}

class PBField:
	extends RefCounted
	func _init(a_name : String, a_type : int, a_rule : int, a_tag : int, packed : bool, a_value = null):
		name = a_name
		type = a_type
		rule = a_rule
		tag = a_tag
		option_packed = packed
		value = a_value

	var name : String
	var type : int
	var rule : int
	var tag : int
	var option_packed : bool
	var value
	var is_map_field : bool = false
	var option_default : bool = false

class PBTypeTag:
	extends RefCounted
	var ok : bool = false
	var type : int
	var tag : int
	var offset : int

class PBServiceField:
	extends RefCounted
	var field : PBField
	var func_ref = null
	var state : int = PB_SERVICE_STATE.UNFILLED

class PBPacker:
	static func convert_signed(n : int) -> int:
		if n < -2147483648:
			return (n << 1) ^ (n >> 63)
		else:
			return (n << 1) ^ (n >> 31)

	static func deconvert_signed(n : int) -> int:
		if n & 0x01:
			return ~(n >> 1)
		else:
			return (n >> 1)

	static func pack_varint(value) -> PackedByteArray:
		var varint : PackedByteArray = PackedByteArray()
		if typeof(value) == TYPE_BOOL:
			if value:
				value = 1
			else:
				value = 0
		for _i in range(9):
			var b = value & 0x7F
			value >>= 7
			if value:
				varint.append(b | 0x80)
			else:
				varint.append(b)
				break
		if varint.size() == 9 && (varint[8] & 0x80 != 0):
			varint.append(0x01)
		return varint

	static func pack_bytes(value, count : int, data_type : int) -> PackedByteArray:
		var bytes : PackedByteArray = PackedByteArray()
		if data_type == PB_DATA_TYPE.FLOAT:
			var spb : StreamPeerBuffer = StreamPeerBuffer.new()
			spb.put_float(value)
			bytes = spb.get_data_array()
		elif data_type == PB_DATA_TYPE.DOUBLE:
			var spb : StreamPeerBuffer = StreamPeerBuffer.new()
			spb.put_double(value)
			bytes = spb.get_data_array()
		else:
			for _i in range(count):
				bytes.append(value & 0xFF)
				value >>= 8
		return bytes

	static func unpack_bytes(bytes : PackedByteArray, index : int, count : int, data_type : int):
		if data_type == PB_DATA_TYPE.FLOAT:
			return bytes.decode_float(index)
		elif data_type == PB_DATA_TYPE.DOUBLE:
			return bytes.decode_double(index)
		elif data_type == PB_DATA_TYPE.FIXED32:
			return bytes.decode_u32(index)
		elif data_type == PB_DATA_TYPE.SFIXED32:
			return bytes.decode_s32(index)
		elif data_type == PB_DATA_TYPE.FIXED64:
			return bytes.decode_u64(index)
		elif data_type == PB_DATA_TYPE.SFIXED64:
			return bytes.decode_s64(index)
		else:
			var value : int = 0
			for i in range(count):
				value |= bytes[index + i] << (8 * i)
			return value

	static func unpack_varint(varint_bytes) -> int:
		var value : int = 0
		var i: int = varint_bytes.size() - 1
		while i > -1:
			value = (value << 7) | (varint_bytes[i] & 0x7F)
			i -= 1
		return value

	static func pack_type_tag(type : int, tag : int) -> PackedByteArray:
		return pack_varint((tag << 3) | type)

	static func isolate_varint(bytes : PackedByteArray, index : int) -> PackedByteArray:
		var i: int = index
		while i <= index + 10 && i < bytes.size(): # Protobuf varint max size is 10 bytes
			if !(bytes[i] & 0x80):
				return bytes.slice(index, i + 1)
			i += 1
		return [] # Unreachable

	static func unpack_type_tag(bytes : PackedByteArray, index : int) -> PBTypeTag:
		var varint_bytes : PackedByteArray = isolate_varint(bytes, index)
		var result : PBTypeTag = PBTypeTag.new()
		if varint_bytes.size() != 0:
			result.ok = true
			result.offset = varint_bytes.size()
			var unpacked : int = unpack_varint(varint_bytes)
			result.type = unpacked & 0x07
			result.tag = unpacked >> 3
		return result

	static func pack_length_delimeted(type : int, tag : int, bytes : PackedByteArray) -> PackedByteArray:
		var result : PackedByteArray = pack_type_tag(type, tag)
		result.append_array(pack_varint(bytes.size()))
		result.append_array(bytes)
		return result

	static func pb_type_from_data_type(data_type : int) -> int:
		if data_type == PB_DATA_TYPE.INT32 || data_type == PB_DATA_TYPE.SINT32 || data_type == PB_DATA_TYPE.UINT32 || data_type == PB_DATA_TYPE.INT64 || data_type == PB_DATA_TYPE.SINT64 || data_type == PB_DATA_TYPE.UINT64 || data_type == PB_DATA_TYPE.BOOL || data_type == PB_DATA_TYPE.ENUM:
			return PB_TYPE.VARINT
		elif data_type == PB_DATA_TYPE.FIXED32 || data_type == PB_DATA_TYPE.SFIXED32 || data_type == PB_DATA_TYPE.FLOAT:
			return PB_TYPE.FIX32
		elif data_type == PB_DATA_TYPE.FIXED64 || data_type == PB_DATA_TYPE.SFIXED64 || data_type == PB_DATA_TYPE.DOUBLE:
			return PB_TYPE.FIX64
		elif data_type == PB_DATA_TYPE.STRING || data_type == PB_DATA_TYPE.BYTES || data_type == PB_DATA_TYPE.MESSAGE || data_type == PB_DATA_TYPE.MAP:
			return PB_TYPE.LENGTHDEL
		else:
			return PB_TYPE.UNDEFINED

	static func pack_field(field : PBField) -> PackedByteArray:
		var type : int = pb_type_from_data_type(field.type)
		var type_copy : int = type
		if field.rule == PB_RULE.REPEATED && field.option_packed:
			type = PB_TYPE.LENGTHDEL
		var head : PackedByteArray = pack_type_tag(type, field.tag)
		var data : PackedByteArray = PackedByteArray()
		if type == PB_TYPE.VARINT:
			var value
			if field.rule == PB_RULE.REPEATED:
				for v in field.value:
					data.append_array(head)
					if field.type == PB_DATA_TYPE.SINT32 || field.type == PB_DATA_TYPE.SINT64:
						value = convert_signed(v)
					else:
						value = v
					data.append_array(pack_varint(value))
				return data
			else:
				if field.type == PB_DATA_TYPE.SINT32 || field.type == PB_DATA_TYPE.SINT64:
					value = convert_signed(field.value)
				else:
					value = field.value
				data = pack_varint(value)
		elif type == PB_TYPE.FIX32:
			if field.rule == PB_RULE.REPEATED:
				for v in field.value:
					data.append_array(head)
					data.append_array(pack_bytes(v, 4, field.type))
				return data
			else:
				data.append_array(pack_bytes(field.value, 4, field.type))
		elif type == PB_TYPE.FIX64:
			if field.rule == PB_RULE.REPEATED:
				for v in field.value:
					data.append_array(head)
					data.append_array(pack_bytes(v, 8, field.type))
				return data
			else:
				data.append_array(pack_bytes(field.value, 8, field.type))
		elif type == PB_TYPE.LENGTHDEL:
			if field.rule == PB_RULE.REPEATED:
				if type_copy == PB_TYPE.VARINT:
					if field.type == PB_DATA_TYPE.SINT32 || field.type == PB_DATA_TYPE.SINT64:
						var signed_value : int
						for v in field.value:
							signed_value = convert_signed(v)
							data.append_array(pack_varint(signed_value))
					else:
						for v in field.value:
							data.append_array(pack_varint(v))
					return pack_length_delimeted(type, field.tag, data)
				elif type_copy == PB_TYPE.FIX32:
					for v in field.value:
						data.append_array(pack_bytes(v, 4, field.type))
					return pack_length_delimeted(type, field.tag, data)
				elif type_copy == PB_TYPE.FIX64:
					for v in field.value:
						data.append_array(pack_bytes(v, 8, field.type))
					return pack_length_delimeted(type, field.tag, data)
				elif field.type == PB_DATA_TYPE.STRING:
					for v in field.value:
						var obj = v.to_utf8_buffer()
						data.append_array(pack_length_delimeted(type, field.tag, obj))
					return data
				elif field.type == PB_DATA_TYPE.BYTES:
					for v in field.value:
						data.append_array(pack_length_delimeted(type, field.tag, v))
					return data
				elif typeof(field.value[0]) == TYPE_OBJECT:
					for v in field.value:
						var obj : PackedByteArray = v.to_bytes()
						data.append_array(pack_length_delimeted(type, field.tag, obj))
					return data
			else:
				if field.type == PB_DATA_TYPE.STRING:
					var str_bytes : PackedByteArray = field.value.to_utf8_buffer()
					if PROTO_VERSION == 2 || (PROTO_VERSION == 3 && str_bytes.size() > 0):
						data.append_array(str_bytes)
						return pack_length_delimeted(type, field.tag, data)
				if field.type == PB_DATA_TYPE.BYTES:
					if PROTO_VERSION == 2 || (PROTO_VERSION == 3 && field.value.size() > 0):
						data.append_array(field.value)
						return pack_length_delimeted(type, field.tag, data)
				elif typeof(field.value) == TYPE_OBJECT:
					var obj : PackedByteArray = field.value.to_bytes()
					if obj.size() > 0:
						data.append_array(obj)
					return pack_length_delimeted(type, field.tag, data)
				else:
					pass
		if data.size() > 0:
			head.append_array(data)
			return head
		else:
			return data

	static func skip_unknown_field(bytes : PackedByteArray, offset : int, type : int) -> int:
		if type == PB_TYPE.VARINT:
			return offset + isolate_varint(bytes, offset).size()
		if type == PB_TYPE.FIX64:
			return offset + 8
		if type == PB_TYPE.LENGTHDEL:
			var length_bytes : PackedByteArray = isolate_varint(bytes, offset)
			var length : int = unpack_varint(length_bytes)
			return offset + length_bytes.size() + length
		if type == PB_TYPE.FIX32:
			return offset + 4
		return PB_ERR.UNDEFINED_STATE

	static func unpack_field(bytes : PackedByteArray, offset : int, field : PBField, type : int, message_func_ref) -> int:
		if field.rule == PB_RULE.REPEATED && type != PB_TYPE.LENGTHDEL && field.option_packed:
			var count = isolate_varint(bytes, offset)
			if count.size() > 0:
				offset += count.size()
				count = unpack_varint(count)
				if type == PB_TYPE.VARINT:
					var val
					var counter = offset + count
					while offset < counter:
						val = isolate_varint(bytes, offset)
						if val.size() > 0:
							offset += val.size()
							val = unpack_varint(val)
							if field.type == PB_DATA_TYPE.SINT32 || field.type == PB_DATA_TYPE.SINT64:
								val = deconvert_signed(val)
							elif field.type == PB_DATA_TYPE.BOOL:
								if val:
									val = true
								else:
									val = false
							field.value.append(val)
						else:
							return PB_ERR.REPEATED_COUNT_MISMATCH
					return offset
				elif type == PB_TYPE.FIX32 || type == PB_TYPE.FIX64:
					var type_size
					if type == PB_TYPE.FIX32:
						type_size = 4
					else:
						type_size = 8
					var val
					var counter = offset + count
					while offset < counter:
						if (offset + type_size) > bytes.size():
							return PB_ERR.REPEATED_COUNT_MISMATCH
						val = unpack_bytes(bytes, offset, type_size, field.type)
						offset += type_size
						field.value.append(val)
					return offset
			else:
				return PB_ERR.REPEATED_COUNT_NOT_FOUND
		else:
			if type == PB_TYPE.VARINT:
				var val = isolate_varint(bytes, offset)
				if val.size() > 0:
					offset += val.size()
					val = unpack_varint(val)
					if field.type == PB_DATA_TYPE.SINT32 || field.type == PB_DATA_TYPE.SINT64:
						val = deconvert_signed(val)
					elif field.type == PB_DATA_TYPE.BOOL:
						if val:
							val = true
						else:
							val = false
					if field.rule == PB_RULE.REPEATED:
						field.value.append(val)
					else:
						field.value = val
				else:
					return PB_ERR.VARINT_NOT_FOUND
				return offset
			elif type == PB_TYPE.FIX32 || type == PB_TYPE.FIX64:
				var type_size
				if type == PB_TYPE.FIX32:
					type_size = 4
				else:
					type_size = 8
				var val
				if (offset + type_size) > bytes.size():
					return PB_ERR.REPEATED_COUNT_MISMATCH
				val = unpack_bytes(bytes, offset, type_size, field.type)
				offset += type_size
				if field.rule == PB_RULE.REPEATED:
					field.value.append(val)
				else:
					field.value = val
				return offset
			elif type == PB_TYPE.LENGTHDEL:
				var inner_size = isolate_varint(bytes, offset)
				if inner_size.size() > 0:
					offset += inner_size.size()
					inner_size = unpack_varint(inner_size)
					if inner_size >= 0:
						if inner_size + offset > bytes.size():
							return PB_ERR.LENGTHDEL_SIZE_MISMATCH
						if message_func_ref != null:
							var message = message_func_ref.call()
							if inner_size > 0:
								var sub_offset = message.from_bytes(bytes, offset, inner_size + offset)
								if sub_offset > 0:
									if sub_offset - offset >= inner_size:
										offset = sub_offset
										return offset
									else:
										return PB_ERR.LENGTHDEL_SIZE_MISMATCH
								return sub_offset
							else:
								return offset
						elif field.type == PB_DATA_TYPE.STRING:
							var str_bytes : PackedByteArray = bytes.slice(offset, inner_size + offset)
							if field.rule == PB_RULE.REPEATED:
								field.value.append(str_bytes.get_string_from_utf8())
							else:
								field.value = str_bytes.get_string_from_utf8()
							return offset + inner_size
						elif field.type == PB_DATA_TYPE.BYTES:
							var val_bytes : PackedByteArray = bytes.slice(offset, inner_size + offset)
							if field.rule == PB_RULE.REPEATED:
								field.value.append(val_bytes)
							else:
								field.value = val_bytes
							return offset + inner_size
					else:
						return PB_ERR.LENGTHDEL_SIZE_NOT_FOUND
				else:
					return PB_ERR.LENGTHDEL_SIZE_NOT_FOUND
		return PB_ERR.UNDEFINED_STATE

	static func unpack_message(data, bytes : PackedByteArray, offset : int, limit : int) -> int:
		while true:
			var tt : PBTypeTag = unpack_type_tag(bytes, offset)
			if tt.ok:
				offset += tt.offset
				if data.has(tt.tag):
					var service : PBServiceField = data[tt.tag]
					var type : int = pb_type_from_data_type(service.field.type)
					if type == tt.type || (tt.type == PB_TYPE.LENGTHDEL && service.field.rule == PB_RULE.REPEATED && service.field.option_packed):
						var res : int = unpack_field(bytes, offset, service.field, type, service.func_ref)
						if res > 0:
							service.state = PB_SERVICE_STATE.FILLED
							offset = res
							if offset == limit:
								return offset
							elif offset > limit:
								return PB_ERR.PACKAGE_SIZE_MISMATCH
						elif res < 0:
							return res
						else:
							break
				else:
					var res : int = skip_unknown_field(bytes, offset, tt.type)
					if res > 0:
						offset = res
						if offset == limit:
							return offset
						elif offset > limit:
							return PB_ERR.PACKAGE_SIZE_MISMATCH
					elif res < 0:
						return res
					else:
						break
			else:
				return offset
		return PB_ERR.UNDEFINED_STATE

	static func pack_message(data) -> PackedByteArray:
		var DEFAULT_VALUES
		if PROTO_VERSION == 2:
			DEFAULT_VALUES = DEFAULT_VALUES_2
		elif PROTO_VERSION == 3:
			DEFAULT_VALUES = DEFAULT_VALUES_3
		var result : PackedByteArray = PackedByteArray()
		var keys : Array = data.keys()
		keys.sort()
		for i in keys:
			if data[i].field.value != null:
				if data[i].state == PB_SERVICE_STATE.UNFILLED \
				&& !data[i].field.is_map_field \
				&& typeof(data[i].field.value) == typeof(DEFAULT_VALUES[data[i].field.type]) \
				&& data[i].field.value == DEFAULT_VALUES[data[i].field.type]:
					continue
				elif data[i].field.rule == PB_RULE.REPEATED && data[i].field.value.size() == 0:
					continue
				result.append_array(pack_field(data[i].field))
			elif data[i].field.rule == PB_RULE.REQUIRED:
				print("Error: required field is not filled: Tag:", data[i].field.tag)
				return PackedByteArray()
		return result

	static func check_required(data) -> bool:
		var keys : Array = data.keys()
		for i in keys:
			if data[i].field.rule == PB_RULE.REQUIRED && data[i].state == PB_SERVICE_STATE.UNFILLED:
				return false
		return true

	static func construct_map(key_values):
		var result = {}
		for kv in key_values:
			result[kv.get_key()] = kv.get_value()
		return result

	static func tabulate(text : String, nesting : int) -> String:
		var tab : String = ""
		for _i in range(nesting):
			tab += DEBUG_TAB
		return tab + text

	static func value_to_string(value, field : PBField, nesting : int) -> String:
		var result : String = ""
		var text : String
		if field.type == PB_DATA_TYPE.MESSAGE:
			result += "{"
			nesting += 1
			text = message_to_string(value.data, nesting)
			if text != "":
				result += "\n" + text
				nesting -= 1
				result += tabulate("}", nesting)
			else:
				nesting -= 1
				result += "}"
		elif field.type == PB_DATA_TYPE.BYTES:
			result += "<"
			for i in range(value.size()):
				result += str(value[i])
				if i != (value.size() - 1):
					result += ", "
			result += ">"
		elif field.type == PB_DATA_TYPE.STRING:
			result += "\"" + value + "\""
		elif field.type == PB_DATA_TYPE.ENUM:
			result += "ENUM::" + str(value)
		else:
			result += str(value)
		return result

	static func field_to_string(field : PBField, nesting : int) -> String:
		var result : String = tabulate(field.name + ": ", nesting)
		if field.type == PB_DATA_TYPE.MAP:
			if field.value.size() > 0:
				result += "(\n"
				nesting += 1
				for i in range(field.value.size()):
					var local_key_value = field.value[i].data[1].field
					result += tabulate(value_to_string(local_key_value.value, local_key_value, nesting), nesting) + ": "
					local_key_value = field.value[i].data[2].field
					result += value_to_string(local_key_value.value, local_key_value, nesting)
					if i != (field.value.size() - 1):
						result += ","
					result += "\n"
				nesting -= 1
				result += tabulate(")", nesting)
			else:
				result += "()"
		elif field.rule == PB_RULE.REPEATED:
			if field.value.size() > 0:
				result += "[\n"
				nesting += 1
				for i in range(field.value.size()):
					result += tabulate(str(i) + ": ", nesting)
					result += value_to_string(field.value[i], field, nesting)
					if i != (field.value.size() - 1):
						result += ","
					result += "\n"
				nesting -= 1
				result += tabulate("]", nesting)
			else:
				result += "[]"
		else:
			result += value_to_string(field.value, field, nesting)
		result += ";\n"
		return result

	static func message_to_string(data, nesting : int = 0) -> String:
		var DEFAULT_VALUES
		if PROTO_VERSION == 2:
			DEFAULT_VALUES = DEFAULT_VALUES_2
		elif PROTO_VERSION == 3:
			DEFAULT_VALUES = DEFAULT_VALUES_3
		var result : String = ""
		var keys : Array = data.keys()
		keys.sort()
		for i in keys:
			if data[i].field.value != null:
				if data[i].state == PB_SERVICE_STATE.UNFILLED \
				&& !data[i].field.is_map_field \
				&& typeof(data[i].field.value) == typeof(DEFAULT_VALUES[data[i].field.type]) \
				&& data[i].field.value == DEFAULT_VALUES[data[i].field.type]:
					continue
				elif data[i].field.rule == PB_RULE.REPEATED && data[i].field.value.size() == 0:
					continue
				result += field_to_string(data[i].field, nesting)
			elif data[i].field.rule == PB_RULE.REQUIRED:
				result += data[i].field.name + ": " + "error"
		return result



############### USER DATA BEGIN ################


class InfernalMonster:
	extends RefCounted
	func _init():
		var service

		__format_version = PBField.new("format_version", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __format_version
		data[__format_version.tag] = service

		__id = PBField.new("id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __id
		data[__id.tag] = service

		__display_name = PBField.new("display_name", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __display_name
		data[__display_name.tag] = service

		__generation = PBField.new("generation", PB_DATA_TYPE.MESSAGE, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE])
		service = PBServiceField.new()
		service.field = __generation
		service.func_ref = Callable(self, "new_generation")
		data[__generation.tag] = service

		__sprites = PBField.new("sprites", PB_DATA_TYPE.MESSAGE, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE])
		service = PBServiceField.new()
		service.field = __sprites
		service.func_ref = Callable(self, "new_sprites")
		data[__sprites.tag] = service

		__physics = PBField.new("physics", PB_DATA_TYPE.MESSAGE, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE])
		service = PBServiceField.new()
		service.field = __physics
		service.func_ref = Callable(self, "new_physics")
		data[__physics.tag] = service

		__gameplay = PBField.new("gameplay", PB_DATA_TYPE.MESSAGE, PB_RULE.OPTIONAL, 7, true, DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE])
		service = PBServiceField.new()
		service.field = __gameplay
		service.func_ref = Callable(self, "new_gameplay")
		data[__gameplay.tag] = service

		var __animations_default: Array[InfernalAnimation] = []
		__animations = PBField.new("animations", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 8, true, __animations_default)
		service = PBServiceField.new()
		service.field = __animations
		service.func_ref = Callable(self, "add_animations")
		data[__animations.tag] = service

		var __attacks_default: Array[InfernalAttack] = []
		__attacks = PBField.new("attacks", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 9, true, __attacks_default)
		service = PBServiceField.new()
		service.field = __attacks
		service.func_ref = Callable(self, "add_attacks")
		data[__attacks.tag] = service

		var __colliders_default: Array[InfernalCollider] = []
		__colliders = PBField.new("colliders", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 10, true, __colliders_default)
		service = PBServiceField.new()
		service.field = __colliders
		service.func_ref = Callable(self, "add_colliders")
		data[__colliders.tag] = service

		var __movement_modes_default: Array[InfernalMovementMode] = []
		__movement_modes = PBField.new("movement_modes", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 11, true, __movement_modes_default)
		service = PBServiceField.new()
		service.field = __movement_modes
		service.func_ref = Callable(self, "add_movement_modes")
		data[__movement_modes.tag] = service

		var __tags_default: Array[String] = []
		__tags = PBField.new("tags", PB_DATA_TYPE.STRING, PB_RULE.REPEATED, 12, true, __tags_default)
		service = PBServiceField.new()
		service.field = __tags
		data[__tags.tag] = service

		__description = PBField.new("description", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 13, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __description
		data[__description.tag] = service

		__behavior = PBField.new("behavior", PB_DATA_TYPE.MESSAGE, PB_RULE.OPTIONAL, 14, true, DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE])
		service = PBServiceField.new()
		service.field = __behavior
		service.func_ref = Callable(self, "new_behavior")
		data[__behavior.tag] = service

		var __projectiles_default: Array[InfernalProjectile] = []
		__projectiles = PBField.new("projectiles", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 15, true, __projectiles_default)
		service = PBServiceField.new()
		service.field = __projectiles
		service.func_ref = Callable(self, "add_projectiles")
		data[__projectiles.tag] = service

		__size = PBField.new("size", PB_DATA_TYPE.MESSAGE, PB_RULE.OPTIONAL, 16, true, DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE])
		service = PBServiceField.new()
		service.field = __size
		service.func_ref = Callable(self, "new_size")
		data[__size.tag] = service

		__mount = PBField.new("mount", PB_DATA_TYPE.MESSAGE, PB_RULE.OPTIONAL, 17, true, DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE])
		service = PBServiceField.new()
		service.field = __mount
		service.func_ref = Callable(self, "new_mount")
		data[__mount.tag] = service

	var data = {}

	var __format_version: PBField
	func has_format_version() -> bool:
		if __format_version.value != null:
			return true
		return false
	func get_format_version() -> int:
		return __format_version.value
	func clear_format_version() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__format_version.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_format_version(value : int) -> void:
		__format_version.value = value

	var __id: PBField
	func has_id() -> bool:
		if __id.value != null:
			return true
		return false
	func get_id() -> String:
		return __id.value
	func clear_id() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_id(value : String) -> void:
		__id.value = value

	var __display_name: PBField
	func has_display_name() -> bool:
		if __display_name.value != null:
			return true
		return false
	func get_display_name() -> String:
		return __display_name.value
	func clear_display_name() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__display_name.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_display_name(value : String) -> void:
		__display_name.value = value

	var __generation: PBField
	func has_generation() -> bool:
		if __generation.value != null:
			return true
		return false
	func get_generation() -> InfernalGenerationInfo:
		return __generation.value
	func clear_generation() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__generation.value = DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE]
	func new_generation() -> InfernalGenerationInfo:
		__generation.value = InfernalGenerationInfo.new()
		return __generation.value

	var __sprites: PBField
	func has_sprites() -> bool:
		if __sprites.value != null:
			return true
		return false
	func get_sprites() -> InfernalSpriteSet:
		return __sprites.value
	func clear_sprites() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__sprites.value = DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE]
	func new_sprites() -> InfernalSpriteSet:
		__sprites.value = InfernalSpriteSet.new()
		return __sprites.value

	var __physics: PBField
	func has_physics() -> bool:
		if __physics.value != null:
			return true
		return false
	func get_physics() -> InfernalPhysicsInfo:
		return __physics.value
	func clear_physics() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__physics.value = DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE]
	func new_physics() -> InfernalPhysicsInfo:
		__physics.value = InfernalPhysicsInfo.new()
		return __physics.value

	var __gameplay: PBField
	func has_gameplay() -> bool:
		if __gameplay.value != null:
			return true
		return false
	func get_gameplay() -> InfernalGameplayInfo:
		return __gameplay.value
	func clear_gameplay() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__gameplay.value = DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE]
	func new_gameplay() -> InfernalGameplayInfo:
		__gameplay.value = InfernalGameplayInfo.new()
		return __gameplay.value

	var __animations: PBField
	func get_animations() -> Array[InfernalAnimation]:
		return __animations.value
	func clear_animations() -> void:
		data[8].state = PB_SERVICE_STATE.UNFILLED
		__animations.value.clear()
	func add_animations() -> InfernalAnimation:
		var element = InfernalAnimation.new()
		__animations.value.append(element)
		return element

	var __attacks: PBField
	func get_attacks() -> Array[InfernalAttack]:
		return __attacks.value
	func clear_attacks() -> void:
		data[9].state = PB_SERVICE_STATE.UNFILLED
		__attacks.value.clear()
	func add_attacks() -> InfernalAttack:
		var element = InfernalAttack.new()
		__attacks.value.append(element)
		return element

	var __colliders: PBField
	func get_colliders() -> Array[InfernalCollider]:
		return __colliders.value
	func clear_colliders() -> void:
		data[10].state = PB_SERVICE_STATE.UNFILLED
		__colliders.value.clear()
	func add_colliders() -> InfernalCollider:
		var element = InfernalCollider.new()
		__colliders.value.append(element)
		return element

	var __movement_modes: PBField
	func get_movement_modes() -> Array[InfernalMovementMode]:
		return __movement_modes.value
	func clear_movement_modes() -> void:
		data[11].state = PB_SERVICE_STATE.UNFILLED
		__movement_modes.value.clear()
	func add_movement_modes() -> InfernalMovementMode:
		var element = InfernalMovementMode.new()
		__movement_modes.value.append(element)
		return element

	var __tags: PBField
	func get_tags() -> Array[String]:
		return __tags.value
	func clear_tags() -> void:
		data[12].state = PB_SERVICE_STATE.UNFILLED
		__tags.value.clear()
	func add_tags(value : String) -> void:
		__tags.value.append(value)

	var __description: PBField
	func has_description() -> bool:
		if __description.value != null:
			return true
		return false
	func get_description() -> String:
		return __description.value
	func clear_description() -> void:
		data[13].state = PB_SERVICE_STATE.UNFILLED
		__description.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_description(value : String) -> void:
		__description.value = value

	var __behavior: PBField
	func has_behavior() -> bool:
		if __behavior.value != null:
			return true
		return false
	func get_behavior() -> InfernalBehaviorInfo:
		return __behavior.value
	func clear_behavior() -> void:
		data[14].state = PB_SERVICE_STATE.UNFILLED
		__behavior.value = DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE]
	func new_behavior() -> InfernalBehaviorInfo:
		__behavior.value = InfernalBehaviorInfo.new()
		return __behavior.value

	var __projectiles: PBField
	func get_projectiles() -> Array[InfernalProjectile]:
		return __projectiles.value
	func clear_projectiles() -> void:
		data[15].state = PB_SERVICE_STATE.UNFILLED
		__projectiles.value.clear()
	func add_projectiles() -> InfernalProjectile:
		var element = InfernalProjectile.new()
		__projectiles.value.append(element)
		return element

	var __size: PBField
	func has_size() -> bool:
		if __size.value != null:
			return true
		return false
	func get_size() -> InfernalSizeInfo:
		return __size.value
	func clear_size() -> void:
		data[16].state = PB_SERVICE_STATE.UNFILLED
		__size.value = DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE]
	func new_size() -> InfernalSizeInfo:
		__size.value = InfernalSizeInfo.new()
		return __size.value

	var __mount: PBField
	func has_mount() -> bool:
		if __mount.value != null:
			return true
		return false
	func get_mount() -> InfernalMountInfo:
		return __mount.value
	func clear_mount() -> void:
		data[17].state = PB_SERVICE_STATE.UNFILLED
		__mount.value = DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE]
	func new_mount() -> InfernalMountInfo:
		__mount.value = InfernalMountInfo.new()
		return __mount.value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalMountInfo:
	extends RefCounted
	func _init():
		var service

		__rider_package = PBField.new("rider_package", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __rider_package
		data[__rider_package.tag] = service

		__mount_package = PBField.new("mount_package", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __mount_package
		data[__mount_package.tag] = service

		__survivor = PBField.new("survivor", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __survivor
		data[__survivor.tag] = service

	var data = {}

	var __rider_package: PBField
	func has_rider_package() -> bool:
		if __rider_package.value != null:
			return true
		return false
	func get_rider_package() -> String:
		return __rider_package.value
	func clear_rider_package() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__rider_package.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_rider_package(value : String) -> void:
		__rider_package.value = value

	var __mount_package: PBField
	func has_mount_package() -> bool:
		if __mount_package.value != null:
			return true
		return false
	func get_mount_package() -> String:
		return __mount_package.value
	func clear_mount_package() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__mount_package.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_mount_package(value : String) -> void:
		__mount_package.value = value

	var __survivor: PBField
	func has_survivor() -> bool:
		if __survivor.value != null:
			return true
		return false
	func get_survivor() -> String:
		return __survivor.value
	func clear_survivor() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__survivor.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_survivor(value : String) -> void:
		__survivor.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalSpawnInfo:
	extends RefCounted
	func _init():
		var service

		__package_path = PBField.new("package_path", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __package_path
		data[__package_path.tag] = service

		__monster_id = PBField.new("monster_id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __monster_id
		data[__monster_id.tag] = service

		__count = PBField.new("count", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __count
		data[__count.tag] = service

		__max_active = PBField.new("max_active", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __max_active
		data[__max_active.tag] = service

	var data = {}

	var __package_path: PBField
	func has_package_path() -> bool:
		if __package_path.value != null:
			return true
		return false
	func get_package_path() -> String:
		return __package_path.value
	func clear_package_path() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__package_path.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_package_path(value : String) -> void:
		__package_path.value = value

	var __monster_id: PBField
	func has_monster_id() -> bool:
		if __monster_id.value != null:
			return true
		return false
	func get_monster_id() -> String:
		return __monster_id.value
	func clear_monster_id() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__monster_id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_monster_id(value : String) -> void:
		__monster_id.value = value

	var __count: PBField
	func has_count() -> bool:
		if __count.value != null:
			return true
		return false
	func get_count() -> int:
		return __count.value
	func clear_count() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__count.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_count(value : int) -> void:
		__count.value = value

	var __max_active: PBField
	func has_max_active() -> bool:
		if __max_active.value != null:
			return true
		return false
	func get_max_active() -> int:
		return __max_active.value
	func clear_max_active() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__max_active.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_max_active(value : int) -> void:
		__max_active.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalSizeInfo:
	extends RefCounted
	func _init():
		var service

		__normalized_size = PBField.new("normalized_size", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __normalized_size
		data[__normalized_size.tag] = service

		__size_class = PBField.new("size_class", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __size_class
		data[__size_class.tag] = service

		__morphology_scale = PBField.new("morphology_scale", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __morphology_scale
		data[__morphology_scale.tag] = service

		__pixels_per_unit = PBField.new("pixels_per_unit", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __pixels_per_unit
		data[__pixels_per_unit.tag] = service

		__visible_width_px = PBField.new("visible_width_px", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __visible_width_px
		data[__visible_width_px.tag] = service

		__visible_height_px = PBField.new("visible_height_px", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __visible_height_px
		data[__visible_height_px.tag] = service

		__anchor_x_px = PBField.new("anchor_x_px", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 7, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __anchor_x_px
		data[__anchor_x_px.tag] = service

		__anchor_y_px = PBField.new("anchor_y_px", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 8, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __anchor_y_px
		data[__anchor_y_px.tag] = service

	var data = {}

	var __normalized_size: PBField
	func has_normalized_size() -> bool:
		if __normalized_size.value != null:
			return true
		return false
	func get_normalized_size() -> float:
		return __normalized_size.value
	func clear_normalized_size() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__normalized_size.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_normalized_size(value : float) -> void:
		__normalized_size.value = value

	var __size_class: PBField
	func has_size_class() -> bool:
		if __size_class.value != null:
			return true
		return false
	func get_size_class() -> String:
		return __size_class.value
	func clear_size_class() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__size_class.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_size_class(value : String) -> void:
		__size_class.value = value

	var __morphology_scale: PBField
	func has_morphology_scale() -> bool:
		if __morphology_scale.value != null:
			return true
		return false
	func get_morphology_scale() -> float:
		return __morphology_scale.value
	func clear_morphology_scale() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__morphology_scale.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_morphology_scale(value : float) -> void:
		__morphology_scale.value = value

	var __pixels_per_unit: PBField
	func has_pixels_per_unit() -> bool:
		if __pixels_per_unit.value != null:
			return true
		return false
	func get_pixels_per_unit() -> float:
		return __pixels_per_unit.value
	func clear_pixels_per_unit() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__pixels_per_unit.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_pixels_per_unit(value : float) -> void:
		__pixels_per_unit.value = value

	var __visible_width_px: PBField
	func has_visible_width_px() -> bool:
		if __visible_width_px.value != null:
			return true
		return false
	func get_visible_width_px() -> int:
		return __visible_width_px.value
	func clear_visible_width_px() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__visible_width_px.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_visible_width_px(value : int) -> void:
		__visible_width_px.value = value

	var __visible_height_px: PBField
	func has_visible_height_px() -> bool:
		if __visible_height_px.value != null:
			return true
		return false
	func get_visible_height_px() -> int:
		return __visible_height_px.value
	func clear_visible_height_px() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__visible_height_px.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_visible_height_px(value : int) -> void:
		__visible_height_px.value = value

	var __anchor_x_px: PBField
	func has_anchor_x_px() -> bool:
		if __anchor_x_px.value != null:
			return true
		return false
	func get_anchor_x_px() -> float:
		return __anchor_x_px.value
	func clear_anchor_x_px() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__anchor_x_px.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_anchor_x_px(value : float) -> void:
		__anchor_x_px.value = value

	var __anchor_y_px: PBField
	func has_anchor_y_px() -> bool:
		if __anchor_y_px.value != null:
			return true
		return false
	func get_anchor_y_px() -> float:
		return __anchor_y_px.value
	func clear_anchor_y_px() -> void:
		data[8].state = PB_SERVICE_STATE.UNFILLED
		__anchor_y_px.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_anchor_y_px(value : float) -> void:
		__anchor_y_px.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalBehaviorInfo:
	extends RefCounted
	func _init():
		var service

		__style = PBField.new("style", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __style
		data[__style.tag] = service

		__aggro_range = PBField.new("aggro_range", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __aggro_range
		data[__aggro_range.tag] = service

		__preferred_range = PBField.new("preferred_range", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __preferred_range
		data[__preferred_range.tag] = service

		__aggression = PBField.new("aggression", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __aggression
		data[__aggression.tag] = service

		__retreat_health_fraction = PBField.new("retreat_health_fraction", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __retreat_health_fraction
		data[__retreat_health_fraction.tag] = service

		__approach_mode = PBField.new("approach_mode", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __approach_mode
		data[__approach_mode.tag] = service

		__escape_mode = PBField.new("escape_mode", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 7, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __escape_mode
		data[__escape_mode.tag] = service

		var __attack_preferences_default: Array[InfernalAttackPreference] = []
		__attack_preferences = PBField.new("attack_preferences", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 8, true, __attack_preferences_default)
		service = PBServiceField.new()
		service.field = __attack_preferences
		service.func_ref = Callable(self, "add_attack_preferences")
		data[__attack_preferences.tag] = service

	var data = {}

	var __style: PBField
	func has_style() -> bool:
		if __style.value != null:
			return true
		return false
	func get_style() -> String:
		return __style.value
	func clear_style() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__style.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_style(value : String) -> void:
		__style.value = value

	var __aggro_range: PBField
	func has_aggro_range() -> bool:
		if __aggro_range.value != null:
			return true
		return false
	func get_aggro_range() -> float:
		return __aggro_range.value
	func clear_aggro_range() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__aggro_range.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_aggro_range(value : float) -> void:
		__aggro_range.value = value

	var __preferred_range: PBField
	func has_preferred_range() -> bool:
		if __preferred_range.value != null:
			return true
		return false
	func get_preferred_range() -> float:
		return __preferred_range.value
	func clear_preferred_range() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__preferred_range.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_preferred_range(value : float) -> void:
		__preferred_range.value = value

	var __aggression: PBField
	func has_aggression() -> bool:
		if __aggression.value != null:
			return true
		return false
	func get_aggression() -> float:
		return __aggression.value
	func clear_aggression() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__aggression.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_aggression(value : float) -> void:
		__aggression.value = value

	var __retreat_health_fraction: PBField
	func has_retreat_health_fraction() -> bool:
		if __retreat_health_fraction.value != null:
			return true
		return false
	func get_retreat_health_fraction() -> float:
		return __retreat_health_fraction.value
	func clear_retreat_health_fraction() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__retreat_health_fraction.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_retreat_health_fraction(value : float) -> void:
		__retreat_health_fraction.value = value

	var __approach_mode: PBField
	func has_approach_mode() -> bool:
		if __approach_mode.value != null:
			return true
		return false
	func get_approach_mode() -> String:
		return __approach_mode.value
	func clear_approach_mode() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__approach_mode.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_approach_mode(value : String) -> void:
		__approach_mode.value = value

	var __escape_mode: PBField
	func has_escape_mode() -> bool:
		if __escape_mode.value != null:
			return true
		return false
	func get_escape_mode() -> String:
		return __escape_mode.value
	func clear_escape_mode() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__escape_mode.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_escape_mode(value : String) -> void:
		__escape_mode.value = value

	var __attack_preferences: PBField
	func get_attack_preferences() -> Array[InfernalAttackPreference]:
		return __attack_preferences.value
	func clear_attack_preferences() -> void:
		data[8].state = PB_SERVICE_STATE.UNFILLED
		__attack_preferences.value.clear()
	func add_attack_preferences() -> InfernalAttackPreference:
		var element = InfernalAttackPreference.new()
		__attack_preferences.value.append(element)
		return element

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalAttackPreference:
	extends RefCounted
	func _init():
		var service

		__attack_id = PBField.new("attack_id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __attack_id
		data[__attack_id.tag] = service

		__weight = PBField.new("weight", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __weight
		data[__weight.tag] = service

	var data = {}

	var __attack_id: PBField
	func has_attack_id() -> bool:
		if __attack_id.value != null:
			return true
		return false
	func get_attack_id() -> String:
		return __attack_id.value
	func clear_attack_id() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__attack_id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_attack_id(value : String) -> void:
		__attack_id.value = value

	var __weight: PBField
	func has_weight() -> bool:
		if __weight.value != null:
			return true
		return false
	func get_weight() -> float:
		return __weight.value
	func clear_weight() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__weight.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_weight(value : float) -> void:
		__weight.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalGenerationInfo:
	extends RefCounted
	func _init():
		var service

		__generator_version = PBField.new("generator_version", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __generator_version
		data[__generator_version.tag] = service

		__recipe_version = PBField.new("recipe_version", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __recipe_version
		data[__recipe_version.tag] = service

		__prompt = PBField.new("prompt", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __prompt
		data[__prompt.tag] = service

		__seed = PBField.new("seed", PB_DATA_TYPE.UINT64, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT64])
		service = PBServiceField.new()
		service.field = __seed
		data[__seed.tag] = service

		__parser = PBField.new("parser", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __parser
		data[__parser.tag] = service

		__parser_confidence = PBField.new("parser_confidence", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __parser_confidence
		data[__parser_confidence.tag] = service

		var __repairs_default: Array[String] = []
		__repairs = PBField.new("repairs", PB_DATA_TYPE.STRING, PB_RULE.REPEATED, 7, true, __repairs_default)
		service = PBServiceField.new()
		service.field = __repairs
		data[__repairs.tag] = service

		var __recipe_ids_default: Array[String] = []
		__recipe_ids = PBField.new("recipe_ids", PB_DATA_TYPE.STRING, PB_RULE.REPEATED, 8, true, __recipe_ids_default)
		service = PBServiceField.new()
		service.field = __recipe_ids
		data[__recipe_ids.tag] = service

		var __confidences_default: Array[InfernalSemanticConfidence] = []
		__confidences = PBField.new("confidences", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 9, true, __confidences_default)
		service = PBServiceField.new()
		service.field = __confidences
		service.func_ref = Callable(self, "add_confidences")
		data[__confidences.tag] = service

		var __token_evidence_default: Array[InfernalTokenEvidence] = []
		__token_evidence = PBField.new("token_evidence", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 10, true, __token_evidence_default)
		service = PBServiceField.new()
		service.field = __token_evidence
		service.func_ref = Callable(self, "add_token_evidence")
		data[__token_evidence.tag] = service

	var data = {}

	var __generator_version: PBField
	func has_generator_version() -> bool:
		if __generator_version.value != null:
			return true
		return false
	func get_generator_version() -> String:
		return __generator_version.value
	func clear_generator_version() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__generator_version.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_generator_version(value : String) -> void:
		__generator_version.value = value

	var __recipe_version: PBField
	func has_recipe_version() -> bool:
		if __recipe_version.value != null:
			return true
		return false
	func get_recipe_version() -> String:
		return __recipe_version.value
	func clear_recipe_version() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__recipe_version.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_recipe_version(value : String) -> void:
		__recipe_version.value = value

	var __prompt: PBField
	func has_prompt() -> bool:
		if __prompt.value != null:
			return true
		return false
	func get_prompt() -> String:
		return __prompt.value
	func clear_prompt() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__prompt.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_prompt(value : String) -> void:
		__prompt.value = value

	var __seed: PBField
	func has_seed() -> bool:
		if __seed.value != null:
			return true
		return false
	func get_seed() -> int:
		return __seed.value
	func clear_seed() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__seed.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT64]
	func set_seed(value : int) -> void:
		__seed.value = value

	var __parser: PBField
	func has_parser() -> bool:
		if __parser.value != null:
			return true
		return false
	func get_parser() -> String:
		return __parser.value
	func clear_parser() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__parser.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_parser(value : String) -> void:
		__parser.value = value

	var __parser_confidence: PBField
	func has_parser_confidence() -> bool:
		if __parser_confidence.value != null:
			return true
		return false
	func get_parser_confidence() -> float:
		return __parser_confidence.value
	func clear_parser_confidence() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__parser_confidence.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_parser_confidence(value : float) -> void:
		__parser_confidence.value = value

	var __repairs: PBField
	func get_repairs() -> Array[String]:
		return __repairs.value
	func clear_repairs() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__repairs.value.clear()
	func add_repairs(value : String) -> void:
		__repairs.value.append(value)

	var __recipe_ids: PBField
	func get_recipe_ids() -> Array[String]:
		return __recipe_ids.value
	func clear_recipe_ids() -> void:
		data[8].state = PB_SERVICE_STATE.UNFILLED
		__recipe_ids.value.clear()
	func add_recipe_ids(value : String) -> void:
		__recipe_ids.value.append(value)

	var __confidences: PBField
	func get_confidences() -> Array[InfernalSemanticConfidence]:
		return __confidences.value
	func clear_confidences() -> void:
		data[9].state = PB_SERVICE_STATE.UNFILLED
		__confidences.value.clear()
	func add_confidences() -> InfernalSemanticConfidence:
		var element = InfernalSemanticConfidence.new()
		__confidences.value.append(element)
		return element

	var __token_evidence: PBField
	func get_token_evidence() -> Array[InfernalTokenEvidence]:
		return __token_evidence.value
	func clear_token_evidence() -> void:
		data[10].state = PB_SERVICE_STATE.UNFILLED
		__token_evidence.value.clear()
	func add_token_evidence() -> InfernalTokenEvidence:
		var element = InfernalTokenEvidence.new()
		__token_evidence.value.append(element)
		return element

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalSemanticConfidence:
	extends RefCounted
	func _init():
		var service

		__key = PBField.new("key", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __key
		data[__key.tag] = service

		__confidence = PBField.new("confidence", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __confidence
		data[__confidence.tag] = service

	var data = {}

	var __key: PBField
	func has_key() -> bool:
		if __key.value != null:
			return true
		return false
	func get_key() -> String:
		return __key.value
	func clear_key() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__key.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_key(value : String) -> void:
		__key.value = value

	var __confidence: PBField
	func has_confidence() -> bool:
		if __confidence.value != null:
			return true
		return false
	func get_confidence() -> float:
		return __confidence.value
	func clear_confidence() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__confidence.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_confidence(value : float) -> void:
		__confidence.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalTokenEvidence:
	extends RefCounted
	func _init():
		var service

		__label = PBField.new("label", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __label
		data[__label.tag] = service

		__start = PBField.new("start", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __start
		data[__start.tag] = service

		__end = PBField.new("end", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __end
		data[__end.tag] = service

		__confidence = PBField.new("confidence", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __confidence
		data[__confidence.tag] = service

	var data = {}

	var __label: PBField
	func has_label() -> bool:
		if __label.value != null:
			return true
		return false
	func get_label() -> String:
		return __label.value
	func clear_label() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__label.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_label(value : String) -> void:
		__label.value = value

	var __start: PBField
	func has_start() -> bool:
		if __start.value != null:
			return true
		return false
	func get_start() -> int:
		return __start.value
	func clear_start() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__start.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_start(value : int) -> void:
		__start.value = value

	var __end: PBField
	func has_end() -> bool:
		if __end.value != null:
			return true
		return false
	func get_end() -> int:
		return __end.value
	func clear_end() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__end.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_end(value : int) -> void:
		__end.value = value

	var __confidence: PBField
	func has_confidence() -> bool:
		if __confidence.value != null:
			return true
		return false
	func get_confidence() -> float:
		return __confidence.value
	func clear_confidence() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__confidence.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_confidence(value : float) -> void:
		__confidence.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalSpriteSet:
	extends RefCounted
	func _init():
		var service

		__image_file = PBField.new("image_file", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __image_file
		data[__image_file.tag] = service

		__frame_width = PBField.new("frame_width", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __frame_width
		data[__frame_width.tag] = service

		__frame_height = PBField.new("frame_height", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __frame_height
		data[__frame_height.tag] = service

		__columns = PBField.new("columns", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __columns
		data[__columns.tag] = service

		__frame_count = PBField.new("frame_count", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __frame_count
		data[__frame_count.tag] = service

		__palette_size = PBField.new("palette_size", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __palette_size
		data[__palette_size.tag] = service

		__emission_file = PBField.new("emission_file", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 7, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __emission_file
		data[__emission_file.tag] = service

		var __direction_angles_deg_default: Array[float] = []
		__direction_angles_deg = PBField.new("direction_angles_deg", PB_DATA_TYPE.FLOAT, PB_RULE.REPEATED, 8, true, __direction_angles_deg_default)
		service = PBServiceField.new()
		service.field = __direction_angles_deg
		data[__direction_angles_deg.tag] = service

		__direction_stride = PBField.new("direction_stride", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 9, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __direction_stride
		data[__direction_stride.tag] = service

		__render_mode = PBField.new("render_mode", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 10, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __render_mode
		data[__render_mode.tag] = service

	var data = {}

	var __image_file: PBField
	func has_image_file() -> bool:
		if __image_file.value != null:
			return true
		return false
	func get_image_file() -> String:
		return __image_file.value
	func clear_image_file() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__image_file.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_image_file(value : String) -> void:
		__image_file.value = value

	var __frame_width: PBField
	func has_frame_width() -> bool:
		if __frame_width.value != null:
			return true
		return false
	func get_frame_width() -> int:
		return __frame_width.value
	func clear_frame_width() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__frame_width.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_frame_width(value : int) -> void:
		__frame_width.value = value

	var __frame_height: PBField
	func has_frame_height() -> bool:
		if __frame_height.value != null:
			return true
		return false
	func get_frame_height() -> int:
		return __frame_height.value
	func clear_frame_height() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__frame_height.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_frame_height(value : int) -> void:
		__frame_height.value = value

	var __columns: PBField
	func has_columns() -> bool:
		if __columns.value != null:
			return true
		return false
	func get_columns() -> int:
		return __columns.value
	func clear_columns() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__columns.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_columns(value : int) -> void:
		__columns.value = value

	var __frame_count: PBField
	func has_frame_count() -> bool:
		if __frame_count.value != null:
			return true
		return false
	func get_frame_count() -> int:
		return __frame_count.value
	func clear_frame_count() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__frame_count.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_frame_count(value : int) -> void:
		__frame_count.value = value

	var __palette_size: PBField
	func has_palette_size() -> bool:
		if __palette_size.value != null:
			return true
		return false
	func get_palette_size() -> int:
		return __palette_size.value
	func clear_palette_size() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__palette_size.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_palette_size(value : int) -> void:
		__palette_size.value = value

	var __emission_file: PBField
	func has_emission_file() -> bool:
		if __emission_file.value != null:
			return true
		return false
	func get_emission_file() -> String:
		return __emission_file.value
	func clear_emission_file() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__emission_file.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_emission_file(value : String) -> void:
		__emission_file.value = value

	var __direction_angles_deg: PBField
	func get_direction_angles_deg() -> Array[float]:
		return __direction_angles_deg.value
	func clear_direction_angles_deg() -> void:
		data[8].state = PB_SERVICE_STATE.UNFILLED
		__direction_angles_deg.value.clear()
	func add_direction_angles_deg(value : float) -> void:
		__direction_angles_deg.value.append(value)

	var __direction_stride: PBField
	func has_direction_stride() -> bool:
		if __direction_stride.value != null:
			return true
		return false
	func get_direction_stride() -> int:
		return __direction_stride.value
	func clear_direction_stride() -> void:
		data[9].state = PB_SERVICE_STATE.UNFILLED
		__direction_stride.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_direction_stride(value : int) -> void:
		__direction_stride.value = value

	var __render_mode: PBField
	func has_render_mode() -> bool:
		if __render_mode.value != null:
			return true
		return false
	func get_render_mode() -> String:
		return __render_mode.value
	func clear_render_mode() -> void:
		data[10].state = PB_SERVICE_STATE.UNFILLED
		__render_mode.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_render_mode(value : String) -> void:
		__render_mode.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalPhysicsInfo:
	extends RefCounted
	func _init():
		var service

		__mass = PBField.new("mass", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __mass
		data[__mass.tag] = service

		__gravity_scale = PBField.new("gravity_scale", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __gravity_scale
		data[__gravity_scale.tag] = service

		__speed = PBField.new("speed", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __speed
		data[__speed.tag] = service

		__knockback_scale = PBField.new("knockback_scale", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __knockback_scale
		data[__knockback_scale.tag] = service

		__center_of_mass_x = PBField.new("center_of_mass_x", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __center_of_mass_x
		data[__center_of_mass_x.tag] = service

		__center_of_mass_y = PBField.new("center_of_mass_y", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __center_of_mass_y
		data[__center_of_mass_y.tag] = service

		var __side_effects_default: Array[String] = []
		__side_effects = PBField.new("side_effects", PB_DATA_TYPE.STRING, PB_RULE.REPEATED, 7, true, __side_effects_default)
		service = PBServiceField.new()
		service.field = __side_effects
		data[__side_effects.tag] = service

	var data = {}

	var __mass: PBField
	func has_mass() -> bool:
		if __mass.value != null:
			return true
		return false
	func get_mass() -> float:
		return __mass.value
	func clear_mass() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__mass.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_mass(value : float) -> void:
		__mass.value = value

	var __gravity_scale: PBField
	func has_gravity_scale() -> bool:
		if __gravity_scale.value != null:
			return true
		return false
	func get_gravity_scale() -> float:
		return __gravity_scale.value
	func clear_gravity_scale() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__gravity_scale.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_gravity_scale(value : float) -> void:
		__gravity_scale.value = value

	var __speed: PBField
	func has_speed() -> bool:
		if __speed.value != null:
			return true
		return false
	func get_speed() -> float:
		return __speed.value
	func clear_speed() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__speed.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_speed(value : float) -> void:
		__speed.value = value

	var __knockback_scale: PBField
	func has_knockback_scale() -> bool:
		if __knockback_scale.value != null:
			return true
		return false
	func get_knockback_scale() -> float:
		return __knockback_scale.value
	func clear_knockback_scale() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__knockback_scale.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_knockback_scale(value : float) -> void:
		__knockback_scale.value = value

	var __center_of_mass_x: PBField
	func has_center_of_mass_x() -> bool:
		if __center_of_mass_x.value != null:
			return true
		return false
	func get_center_of_mass_x() -> float:
		return __center_of_mass_x.value
	func clear_center_of_mass_x() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__center_of_mass_x.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_center_of_mass_x(value : float) -> void:
		__center_of_mass_x.value = value

	var __center_of_mass_y: PBField
	func has_center_of_mass_y() -> bool:
		if __center_of_mass_y.value != null:
			return true
		return false
	func get_center_of_mass_y() -> float:
		return __center_of_mass_y.value
	func clear_center_of_mass_y() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__center_of_mass_y.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_center_of_mass_y(value : float) -> void:
		__center_of_mass_y.value = value

	var __side_effects: PBField
	func get_side_effects() -> Array[String]:
		return __side_effects.value
	func clear_side_effects() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__side_effects.value.clear()
	func add_side_effects(value : String) -> void:
		__side_effects.value.append(value)

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalGameplayInfo:
	extends RefCounted
	func _init():
		var service

		__health = PBField.new("health", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __health
		data[__health.tag] = service

		__defense = PBField.new("defense", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __defense
		data[__defense.tag] = service

		__threat = PBField.new("threat", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __threat
		data[__threat.tag] = service

		__health_size_bonus = PBField.new("health_size_bonus", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __health_size_bonus
		data[__health_size_bonus.tag] = service

		__health_armor_bonus = PBField.new("health_armor_bonus", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __health_armor_bonus
		data[__health_armor_bonus.tag] = service

		__health_magic_bonus = PBField.new("health_magic_bonus", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __health_magic_bonus
		data[__health_magic_bonus.tag] = service

	var data = {}

	var __health: PBField
	func has_health() -> bool:
		if __health.value != null:
			return true
		return false
	func get_health() -> int:
		return __health.value
	func clear_health() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__health.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_health(value : int) -> void:
		__health.value = value

	var __defense: PBField
	func has_defense() -> bool:
		if __defense.value != null:
			return true
		return false
	func get_defense() -> int:
		return __defense.value
	func clear_defense() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__defense.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_defense(value : int) -> void:
		__defense.value = value

	var __threat: PBField
	func has_threat() -> bool:
		if __threat.value != null:
			return true
		return false
	func get_threat() -> float:
		return __threat.value
	func clear_threat() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__threat.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_threat(value : float) -> void:
		__threat.value = value

	var __health_size_bonus: PBField
	func has_health_size_bonus() -> bool:
		if __health_size_bonus.value != null:
			return true
		return false
	func get_health_size_bonus() -> int:
		return __health_size_bonus.value
	func clear_health_size_bonus() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__health_size_bonus.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_health_size_bonus(value : int) -> void:
		__health_size_bonus.value = value

	var __health_armor_bonus: PBField
	func has_health_armor_bonus() -> bool:
		if __health_armor_bonus.value != null:
			return true
		return false
	func get_health_armor_bonus() -> int:
		return __health_armor_bonus.value
	func clear_health_armor_bonus() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__health_armor_bonus.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_health_armor_bonus(value : int) -> void:
		__health_armor_bonus.value = value

	var __health_magic_bonus: PBField
	func has_health_magic_bonus() -> bool:
		if __health_magic_bonus.value != null:
			return true
		return false
	func get_health_magic_bonus() -> int:
		return __health_magic_bonus.value
	func clear_health_magic_bonus() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__health_magic_bonus.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_health_magic_bonus(value : int) -> void:
		__health_magic_bonus.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalAnimation:
	extends RefCounted
	func _init():
		var service

		__id = PBField.new("id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __id
		data[__id.tag] = service

		__semantic_state = PBField.new("semantic_state", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __semantic_state
		data[__semantic_state.tag] = service

		var __frames_default: Array[InfernalAnimationFrame] = []
		__frames = PBField.new("frames", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 3, true, __frames_default)
		service = PBServiceField.new()
		service.field = __frames
		service.func_ref = Callable(self, "add_frames")
		data[__frames.tag] = service

		var __events_default: Array[InfernalAnimationEvent] = []
		__events = PBField.new("events", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 4, true, __events_default)
		service = PBServiceField.new()
		service.field = __events
		service.func_ref = Callable(self, "add_events")
		data[__events.tag] = service

		__looped = PBField.new("looped", PB_DATA_TYPE.BOOL, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.BOOL])
		service = PBServiceField.new()
		service.field = __looped
		data[__looped.tag] = service

		__movement_mode = PBField.new("movement_mode", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __movement_mode
		data[__movement_mode.tag] = service

	var data = {}

	var __id: PBField
	func has_id() -> bool:
		if __id.value != null:
			return true
		return false
	func get_id() -> String:
		return __id.value
	func clear_id() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_id(value : String) -> void:
		__id.value = value

	var __semantic_state: PBField
	func has_semantic_state() -> bool:
		if __semantic_state.value != null:
			return true
		return false
	func get_semantic_state() -> String:
		return __semantic_state.value
	func clear_semantic_state() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__semantic_state.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_semantic_state(value : String) -> void:
		__semantic_state.value = value

	var __frames: PBField
	func get_frames() -> Array[InfernalAnimationFrame]:
		return __frames.value
	func clear_frames() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__frames.value.clear()
	func add_frames() -> InfernalAnimationFrame:
		var element = InfernalAnimationFrame.new()
		__frames.value.append(element)
		return element

	var __events: PBField
	func get_events() -> Array[InfernalAnimationEvent]:
		return __events.value
	func clear_events() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__events.value.clear()
	func add_events() -> InfernalAnimationEvent:
		var element = InfernalAnimationEvent.new()
		__events.value.append(element)
		return element

	var __looped: PBField
	func has_looped() -> bool:
		if __looped.value != null:
			return true
		return false
	func get_looped() -> bool:
		return __looped.value
	func clear_looped() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__looped.value = DEFAULT_VALUES_3[PB_DATA_TYPE.BOOL]
	func set_looped(value : bool) -> void:
		__looped.value = value

	var __movement_mode: PBField
	func has_movement_mode() -> bool:
		if __movement_mode.value != null:
			return true
		return false
	func get_movement_mode() -> String:
		return __movement_mode.value
	func clear_movement_mode() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__movement_mode.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_movement_mode(value : String) -> void:
		__movement_mode.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalAnimationFrame:
	extends RefCounted
	func _init():
		var service

		__sprite_frame_id = PBField.new("sprite_frame_id", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __sprite_frame_id
		data[__sprite_frame_id.tag] = service

		__duration_ms = PBField.new("duration_ms", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __duration_ms
		data[__duration_ms.tag] = service

		__root_dx = PBField.new("root_dx", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __root_dx
		data[__root_dx.tag] = service

		__root_dy = PBField.new("root_dy", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __root_dy
		data[__root_dy.tag] = service

	var data = {}

	var __sprite_frame_id: PBField
	func has_sprite_frame_id() -> bool:
		if __sprite_frame_id.value != null:
			return true
		return false
	func get_sprite_frame_id() -> int:
		return __sprite_frame_id.value
	func clear_sprite_frame_id() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__sprite_frame_id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_sprite_frame_id(value : int) -> void:
		__sprite_frame_id.value = value

	var __duration_ms: PBField
	func has_duration_ms() -> bool:
		if __duration_ms.value != null:
			return true
		return false
	func get_duration_ms() -> int:
		return __duration_ms.value
	func clear_duration_ms() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__duration_ms.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_duration_ms(value : int) -> void:
		__duration_ms.value = value

	var __root_dx: PBField
	func has_root_dx() -> bool:
		if __root_dx.value != null:
			return true
		return false
	func get_root_dx() -> float:
		return __root_dx.value
	func clear_root_dx() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__root_dx.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_root_dx(value : float) -> void:
		__root_dx.value = value

	var __root_dy: PBField
	func has_root_dy() -> bool:
		if __root_dy.value != null:
			return true
		return false
	func get_root_dy() -> float:
		return __root_dy.value
	func clear_root_dy() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__root_dy.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_root_dy(value : float) -> void:
		__root_dy.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalAnimationEvent:
	extends RefCounted
	func _init():
		var service

		__time_ms = PBField.new("time_ms", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __time_ms
		data[__time_ms.tag] = service

		__kind = PBField.new("kind", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __kind
		data[__kind.tag] = service

		__reference_id = PBField.new("reference_id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __reference_id
		data[__reference_id.tag] = service

	var data = {}

	var __time_ms: PBField
	func has_time_ms() -> bool:
		if __time_ms.value != null:
			return true
		return false
	func get_time_ms() -> int:
		return __time_ms.value
	func clear_time_ms() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__time_ms.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_time_ms(value : int) -> void:
		__time_ms.value = value

	var __kind: PBField
	func has_kind() -> bool:
		if __kind.value != null:
			return true
		return false
	func get_kind() -> String:
		return __kind.value
	func clear_kind() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__kind.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_kind(value : String) -> void:
		__kind.value = value

	var __reference_id: PBField
	func has_reference_id() -> bool:
		if __reference_id.value != null:
			return true
		return false
	func get_reference_id() -> String:
		return __reference_id.value
	func clear_reference_id() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__reference_id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_reference_id(value : String) -> void:
		__reference_id.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalAttack:
	extends RefCounted
	func _init():
		var service

		__id = PBField.new("id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __id
		data[__id.tag] = service

		__delivery = PBField.new("delivery", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __delivery
		data[__delivery.tag] = service

		__element = PBField.new("element", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __element
		data[__element.tag] = service

		__origin_node = PBField.new("origin_node", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __origin_node
		data[__origin_node.tag] = service

		__damage = PBField.new("damage", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __damage
		data[__damage.tag] = service

		__telegraph_ms = PBField.new("telegraph_ms", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __telegraph_ms
		data[__telegraph_ms.tag] = service

		__cooldown_ms = PBField.new("cooldown_ms", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 7, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __cooldown_ms
		data[__cooldown_ms.tag] = service

		__range = PBField.new("range", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 8, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __range
		data[__range.tag] = service

		__animation_id = PBField.new("animation_id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 9, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __animation_id
		data[__animation_id.tag] = service

		var __steps_default: Array[InfernalAttackStep] = []
		__steps = PBField.new("steps", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 10, true, __steps_default)
		service = PBServiceField.new()
		service.field = __steps
		service.func_ref = Callable(self, "add_steps")
		data[__steps.tag] = service

		__projectile_id = PBField.new("projectile_id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 11, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __projectile_id
		data[__projectile_id.tag] = service

		__spawn = PBField.new("spawn", PB_DATA_TYPE.MESSAGE, PB_RULE.OPTIONAL, 12, true, DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE])
		service = PBServiceField.new()
		service.field = __spawn
		service.func_ref = Callable(self, "new_spawn")
		data[__spawn.tag] = service

	var data = {}

	var __id: PBField
	func has_id() -> bool:
		if __id.value != null:
			return true
		return false
	func get_id() -> String:
		return __id.value
	func clear_id() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_id(value : String) -> void:
		__id.value = value

	var __delivery: PBField
	func has_delivery() -> bool:
		if __delivery.value != null:
			return true
		return false
	func get_delivery() -> String:
		return __delivery.value
	func clear_delivery() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__delivery.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_delivery(value : String) -> void:
		__delivery.value = value

	var __element: PBField
	func has_element() -> bool:
		if __element.value != null:
			return true
		return false
	func get_element() -> String:
		return __element.value
	func clear_element() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__element.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_element(value : String) -> void:
		__element.value = value

	var __origin_node: PBField
	func has_origin_node() -> bool:
		if __origin_node.value != null:
			return true
		return false
	func get_origin_node() -> String:
		return __origin_node.value
	func clear_origin_node() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__origin_node.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_origin_node(value : String) -> void:
		__origin_node.value = value

	var __damage: PBField
	func has_damage() -> bool:
		if __damage.value != null:
			return true
		return false
	func get_damage() -> int:
		return __damage.value
	func clear_damage() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__damage.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_damage(value : int) -> void:
		__damage.value = value

	var __telegraph_ms: PBField
	func has_telegraph_ms() -> bool:
		if __telegraph_ms.value != null:
			return true
		return false
	func get_telegraph_ms() -> int:
		return __telegraph_ms.value
	func clear_telegraph_ms() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__telegraph_ms.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_telegraph_ms(value : int) -> void:
		__telegraph_ms.value = value

	var __cooldown_ms: PBField
	func has_cooldown_ms() -> bool:
		if __cooldown_ms.value != null:
			return true
		return false
	func get_cooldown_ms() -> int:
		return __cooldown_ms.value
	func clear_cooldown_ms() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__cooldown_ms.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_cooldown_ms(value : int) -> void:
		__cooldown_ms.value = value

	var __range: PBField
	func has_range() -> bool:
		if __range.value != null:
			return true
		return false
	func get_range() -> float:
		return __range.value
	func clear_range() -> void:
		data[8].state = PB_SERVICE_STATE.UNFILLED
		__range.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_range(value : float) -> void:
		__range.value = value

	var __animation_id: PBField
	func has_animation_id() -> bool:
		if __animation_id.value != null:
			return true
		return false
	func get_animation_id() -> String:
		return __animation_id.value
	func clear_animation_id() -> void:
		data[9].state = PB_SERVICE_STATE.UNFILLED
		__animation_id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_animation_id(value : String) -> void:
		__animation_id.value = value

	var __steps: PBField
	func get_steps() -> Array[InfernalAttackStep]:
		return __steps.value
	func clear_steps() -> void:
		data[10].state = PB_SERVICE_STATE.UNFILLED
		__steps.value.clear()
	func add_steps() -> InfernalAttackStep:
		var element = InfernalAttackStep.new()
		__steps.value.append(element)
		return element

	var __projectile_id: PBField
	func has_projectile_id() -> bool:
		if __projectile_id.value != null:
			return true
		return false
	func get_projectile_id() -> String:
		return __projectile_id.value
	func clear_projectile_id() -> void:
		data[11].state = PB_SERVICE_STATE.UNFILLED
		__projectile_id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_projectile_id(value : String) -> void:
		__projectile_id.value = value

	var __spawn: PBField
	func has_spawn() -> bool:
		if __spawn.value != null:
			return true
		return false
	func get_spawn() -> InfernalSpawnInfo:
		return __spawn.value
	func clear_spawn() -> void:
		data[12].state = PB_SERVICE_STATE.UNFILLED
		__spawn.value = DEFAULT_VALUES_3[PB_DATA_TYPE.MESSAGE]
	func new_spawn() -> InfernalSpawnInfo:
		__spawn.value = InfernalSpawnInfo.new()
		return __spawn.value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalProjectile:
	extends RefCounted
	func _init():
		var service

		__id = PBField.new("id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __id
		data[__id.tag] = service

		__kind = PBField.new("kind", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __kind
		data[__kind.tag] = service

		__image_file = PBField.new("image_file", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __image_file
		data[__image_file.tag] = service

		__frame_width = PBField.new("frame_width", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __frame_width
		data[__frame_width.tag] = service

		__frame_height = PBField.new("frame_height", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __frame_height
		data[__frame_height.tag] = service

		__columns = PBField.new("columns", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __columns
		data[__columns.tag] = service

		__first_frame = PBField.new("first_frame", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 7, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __first_frame
		data[__first_frame.tag] = service

		__frame_count = PBField.new("frame_count", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 8, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __frame_count
		data[__frame_count.tag] = service

		__frame_duration_ms = PBField.new("frame_duration_ms", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 9, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __frame_duration_ms
		data[__frame_duration_ms.tag] = service

		__collision_radius = PBField.new("collision_radius", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 10, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __collision_radius
		data[__collision_radius.tag] = service

		__orientation_mode = PBField.new("orientation_mode", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 11, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __orientation_mode
		data[__orientation_mode.tag] = service

	var data = {}

	var __id: PBField
	func has_id() -> bool:
		if __id.value != null:
			return true
		return false
	func get_id() -> String:
		return __id.value
	func clear_id() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_id(value : String) -> void:
		__id.value = value

	var __kind: PBField
	func has_kind() -> bool:
		if __kind.value != null:
			return true
		return false
	func get_kind() -> String:
		return __kind.value
	func clear_kind() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__kind.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_kind(value : String) -> void:
		__kind.value = value

	var __image_file: PBField
	func has_image_file() -> bool:
		if __image_file.value != null:
			return true
		return false
	func get_image_file() -> String:
		return __image_file.value
	func clear_image_file() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__image_file.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_image_file(value : String) -> void:
		__image_file.value = value

	var __frame_width: PBField
	func has_frame_width() -> bool:
		if __frame_width.value != null:
			return true
		return false
	func get_frame_width() -> int:
		return __frame_width.value
	func clear_frame_width() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__frame_width.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_frame_width(value : int) -> void:
		__frame_width.value = value

	var __frame_height: PBField
	func has_frame_height() -> bool:
		if __frame_height.value != null:
			return true
		return false
	func get_frame_height() -> int:
		return __frame_height.value
	func clear_frame_height() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__frame_height.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_frame_height(value : int) -> void:
		__frame_height.value = value

	var __columns: PBField
	func has_columns() -> bool:
		if __columns.value != null:
			return true
		return false
	func get_columns() -> int:
		return __columns.value
	func clear_columns() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__columns.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_columns(value : int) -> void:
		__columns.value = value

	var __first_frame: PBField
	func has_first_frame() -> bool:
		if __first_frame.value != null:
			return true
		return false
	func get_first_frame() -> int:
		return __first_frame.value
	func clear_first_frame() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__first_frame.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_first_frame(value : int) -> void:
		__first_frame.value = value

	var __frame_count: PBField
	func has_frame_count() -> bool:
		if __frame_count.value != null:
			return true
		return false
	func get_frame_count() -> int:
		return __frame_count.value
	func clear_frame_count() -> void:
		data[8].state = PB_SERVICE_STATE.UNFILLED
		__frame_count.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_frame_count(value : int) -> void:
		__frame_count.value = value

	var __frame_duration_ms: PBField
	func has_frame_duration_ms() -> bool:
		if __frame_duration_ms.value != null:
			return true
		return false
	func get_frame_duration_ms() -> int:
		return __frame_duration_ms.value
	func clear_frame_duration_ms() -> void:
		data[9].state = PB_SERVICE_STATE.UNFILLED
		__frame_duration_ms.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_frame_duration_ms(value : int) -> void:
		__frame_duration_ms.value = value

	var __collision_radius: PBField
	func has_collision_radius() -> bool:
		if __collision_radius.value != null:
			return true
		return false
	func get_collision_radius() -> float:
		return __collision_radius.value
	func clear_collision_radius() -> void:
		data[10].state = PB_SERVICE_STATE.UNFILLED
		__collision_radius.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_collision_radius(value : float) -> void:
		__collision_radius.value = value

	var __orientation_mode: PBField
	func has_orientation_mode() -> bool:
		if __orientation_mode.value != null:
			return true
		return false
	func get_orientation_mode() -> String:
		return __orientation_mode.value
	func clear_orientation_mode() -> void:
		data[11].state = PB_SERVICE_STATE.UNFILLED
		__orientation_mode.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_orientation_mode(value : String) -> void:
		__orientation_mode.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalAttackStep:
	extends RefCounted
	func _init():
		var service

		__primitive = PBField.new("primitive", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __primitive
		data[__primitive.tag] = service

		__at_ms = PBField.new("at_ms", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __at_ms
		data[__at_ms.tag] = service

		__value = PBField.new("value", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __value
		data[__value.tag] = service

		__target = PBField.new("target", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __target
		data[__target.tag] = service

		__count = PBField.new("count", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __count
		data[__count.tag] = service

		__spread_deg = PBField.new("spread_deg", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __spread_deg
		data[__spread_deg.tag] = service

		__speed = PBField.new("speed", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 7, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __speed
		data[__speed.tag] = service

		__radius = PBField.new("radius", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 8, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __radius
		data[__radius.tag] = service

		__duration_ms = PBField.new("duration_ms", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 9, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __duration_ms
		data[__duration_ms.tag] = service

	var data = {}

	var __primitive: PBField
	func has_primitive() -> bool:
		if __primitive.value != null:
			return true
		return false
	func get_primitive() -> String:
		return __primitive.value
	func clear_primitive() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__primitive.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_primitive(value : String) -> void:
		__primitive.value = value

	var __at_ms: PBField
	func has_at_ms() -> bool:
		if __at_ms.value != null:
			return true
		return false
	func get_at_ms() -> int:
		return __at_ms.value
	func clear_at_ms() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__at_ms.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_at_ms(value : int) -> void:
		__at_ms.value = value

	var __value: PBField
	func has_value() -> bool:
		if __value.value != null:
			return true
		return false
	func get_value() -> float:
		return __value.value
	func clear_value() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__value.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_value(value : float) -> void:
		__value.value = value

	var __target: PBField
	func has_target() -> bool:
		if __target.value != null:
			return true
		return false
	func get_target() -> String:
		return __target.value
	func clear_target() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__target.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_target(value : String) -> void:
		__target.value = value

	var __count: PBField
	func has_count() -> bool:
		if __count.value != null:
			return true
		return false
	func get_count() -> int:
		return __count.value
	func clear_count() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__count.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_count(value : int) -> void:
		__count.value = value

	var __spread_deg: PBField
	func has_spread_deg() -> bool:
		if __spread_deg.value != null:
			return true
		return false
	func get_spread_deg() -> float:
		return __spread_deg.value
	func clear_spread_deg() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__spread_deg.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_spread_deg(value : float) -> void:
		__spread_deg.value = value

	var __speed: PBField
	func has_speed() -> bool:
		if __speed.value != null:
			return true
		return false
	func get_speed() -> float:
		return __speed.value
	func clear_speed() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__speed.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_speed(value : float) -> void:
		__speed.value = value

	var __radius: PBField
	func has_radius() -> bool:
		if __radius.value != null:
			return true
		return false
	func get_radius() -> float:
		return __radius.value
	func clear_radius() -> void:
		data[8].state = PB_SERVICE_STATE.UNFILLED
		__radius.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_radius(value : float) -> void:
		__radius.value = value

	var __duration_ms: PBField
	func has_duration_ms() -> bool:
		if __duration_ms.value != null:
			return true
		return false
	func get_duration_ms() -> int:
		return __duration_ms.value
	func clear_duration_ms() -> void:
		data[9].state = PB_SERVICE_STATE.UNFILLED
		__duration_ms.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_duration_ms(value : int) -> void:
		__duration_ms.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalCollider:
	extends RefCounted
	func _init():
		var service

		__id = PBField.new("id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __id
		data[__id.tag] = service

		__node_id = PBField.new("node_id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __node_id
		data[__node_id.tag] = service

		__x = PBField.new("x", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __x
		data[__x.tag] = service

		__y = PBField.new("y", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __y
		data[__y.tag] = service

		__radius = PBField.new("radius", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 5, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __radius
		data[__radius.tag] = service

		__hurtbox = PBField.new("hurtbox", PB_DATA_TYPE.BOOL, PB_RULE.OPTIONAL, 6, true, DEFAULT_VALUES_3[PB_DATA_TYPE.BOOL])
		service = PBServiceField.new()
		service.field = __hurtbox
		data[__hurtbox.tag] = service

		var __views_default: Array[InfernalColliderView] = []
		__views = PBField.new("views", PB_DATA_TYPE.MESSAGE, PB_RULE.REPEATED, 7, true, __views_default)
		service = PBServiceField.new()
		service.field = __views
		service.func_ref = Callable(self, "add_views")
		data[__views.tag] = service

	var data = {}

	var __id: PBField
	func has_id() -> bool:
		if __id.value != null:
			return true
		return false
	func get_id() -> String:
		return __id.value
	func clear_id() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_id(value : String) -> void:
		__id.value = value

	var __node_id: PBField
	func has_node_id() -> bool:
		if __node_id.value != null:
			return true
		return false
	func get_node_id() -> String:
		return __node_id.value
	func clear_node_id() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__node_id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_node_id(value : String) -> void:
		__node_id.value = value

	var __x: PBField
	func has_x() -> bool:
		if __x.value != null:
			return true
		return false
	func get_x() -> float:
		return __x.value
	func clear_x() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__x.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_x(value : float) -> void:
		__x.value = value

	var __y: PBField
	func has_y() -> bool:
		if __y.value != null:
			return true
		return false
	func get_y() -> float:
		return __y.value
	func clear_y() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__y.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_y(value : float) -> void:
		__y.value = value

	var __radius: PBField
	func has_radius() -> bool:
		if __radius.value != null:
			return true
		return false
	func get_radius() -> float:
		return __radius.value
	func clear_radius() -> void:
		data[5].state = PB_SERVICE_STATE.UNFILLED
		__radius.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_radius(value : float) -> void:
		__radius.value = value

	var __hurtbox: PBField
	func has_hurtbox() -> bool:
		if __hurtbox.value != null:
			return true
		return false
	func get_hurtbox() -> bool:
		return __hurtbox.value
	func clear_hurtbox() -> void:
		data[6].state = PB_SERVICE_STATE.UNFILLED
		__hurtbox.value = DEFAULT_VALUES_3[PB_DATA_TYPE.BOOL]
	func set_hurtbox(value : bool) -> void:
		__hurtbox.value = value

	var __views: PBField
	func get_views() -> Array[InfernalColliderView]:
		return __views.value
	func clear_views() -> void:
		data[7].state = PB_SERVICE_STATE.UNFILLED
		__views.value.clear()
	func add_views() -> InfernalColliderView:
		var element = InfernalColliderView.new()
		__views.value.append(element)
		return element

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalColliderView:
	extends RefCounted
	func _init():
		var service

		__direction_index = PBField.new("direction_index", PB_DATA_TYPE.UINT32, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32])
		service = PBServiceField.new()
		service.field = __direction_index
		data[__direction_index.tag] = service

		__x = PBField.new("x", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __x
		data[__x.tag] = service

		__y = PBField.new("y", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __y
		data[__y.tag] = service

		__radius = PBField.new("radius", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 4, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __radius
		data[__radius.tag] = service

	var data = {}

	var __direction_index: PBField
	func has_direction_index() -> bool:
		if __direction_index.value != null:
			return true
		return false
	func get_direction_index() -> int:
		return __direction_index.value
	func clear_direction_index() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__direction_index.value = DEFAULT_VALUES_3[PB_DATA_TYPE.UINT32]
	func set_direction_index(value : int) -> void:
		__direction_index.value = value

	var __x: PBField
	func has_x() -> bool:
		if __x.value != null:
			return true
		return false
	func get_x() -> float:
		return __x.value
	func clear_x() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__x.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_x(value : float) -> void:
		__x.value = value

	var __y: PBField
	func has_y() -> bool:
		if __y.value != null:
			return true
		return false
	func get_y() -> float:
		return __y.value
	func clear_y() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__y.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_y(value : float) -> void:
		__y.value = value

	var __radius: PBField
	func has_radius() -> bool:
		if __radius.value != null:
			return true
		return false
	func get_radius() -> float:
		return __radius.value
	func clear_radius() -> void:
		data[4].state = PB_SERVICE_STATE.UNFILLED
		__radius.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_radius(value : float) -> void:
		__radius.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

class InfernalMovementMode:
	extends RefCounted
	func _init():
		var service

		__id = PBField.new("id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 1, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __id
		data[__id.tag] = service

		__speed = PBField.new("speed", PB_DATA_TYPE.FLOAT, PB_RULE.OPTIONAL, 2, true, DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT])
		service = PBServiceField.new()
		service.field = __speed
		data[__speed.tag] = service

		__animation_id = PBField.new("animation_id", PB_DATA_TYPE.STRING, PB_RULE.OPTIONAL, 3, true, DEFAULT_VALUES_3[PB_DATA_TYPE.STRING])
		service = PBServiceField.new()
		service.field = __animation_id
		data[__animation_id.tag] = service

	var data = {}

	var __id: PBField
	func has_id() -> bool:
		if __id.value != null:
			return true
		return false
	func get_id() -> String:
		return __id.value
	func clear_id() -> void:
		data[1].state = PB_SERVICE_STATE.UNFILLED
		__id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_id(value : String) -> void:
		__id.value = value

	var __speed: PBField
	func has_speed() -> bool:
		if __speed.value != null:
			return true
		return false
	func get_speed() -> float:
		return __speed.value
	func clear_speed() -> void:
		data[2].state = PB_SERVICE_STATE.UNFILLED
		__speed.value = DEFAULT_VALUES_3[PB_DATA_TYPE.FLOAT]
	func set_speed(value : float) -> void:
		__speed.value = value

	var __animation_id: PBField
	func has_animation_id() -> bool:
		if __animation_id.value != null:
			return true
		return false
	func get_animation_id() -> String:
		return __animation_id.value
	func clear_animation_id() -> void:
		data[3].state = PB_SERVICE_STATE.UNFILLED
		__animation_id.value = DEFAULT_VALUES_3[PB_DATA_TYPE.STRING]
	func set_animation_id(value : String) -> void:
		__animation_id.value = value

	func _to_string() -> String:
		return PBPacker.message_to_string(data)

	func to_bytes() -> PackedByteArray:
		return PBPacker.pack_message(data)

	func from_bytes(bytes : PackedByteArray, offset : int = 0, limit : int = -1) -> int:
		var cur_limit = bytes.size()
		if limit != -1:
			cur_limit = limit
		var result = PBPacker.unpack_message(data, bytes, offset, cur_limit)
		if result == cur_limit:
			if PBPacker.check_required(data):
				if limit == -1:
					return PB_ERR.NO_ERRORS
			else:
				return PB_ERR.REQUIRED_FIELDS
		elif limit == -1 && result > 0:
			return PB_ERR.PARSE_INCOMPLETE
		return result

################ USER DATA END #################
