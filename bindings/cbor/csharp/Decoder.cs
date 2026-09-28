using Spotflow.Cbor;

namespace InfernalDiffusion.Cbor;

public static class Decoder
{
    public static Monster Decode(byte[] data)
    {
        var monster = CborSerializer.Deserialize<Monster>(data)
            ?? throw new InvalidDataException("CBOR contains no monster");
        if (monster.FormatVersion is < 2 or > 7)
            throw new InvalidDataException($"Unsupported monster format_version {monster.FormatVersion}");
        return monster;
    }

    public static Monster Load(string path) => Decode(File.ReadAllBytes(path));
}
