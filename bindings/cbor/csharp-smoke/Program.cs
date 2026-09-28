using InfernalDiffusion.Cbor;

if (args.Length != 1)
    throw new ArgumentException("Pass the path to monster.cbor");
Monster monster = Decoder.Load(args[0]);
if (string.IsNullOrEmpty(monster.ID) || monster.Animations.Count == 0 ||
    monster.Gameplay is null || monster.Gameplay.Health == 0 || monster.Generation?.Seed != 42 ||
    monster.Projectiles.Count == 0 || !monster.Attacks.Any(attack => attack.Spawn is not null))
    throw new InvalidDataException("Decoded monster is incomplete");
Console.WriteLine($"{monster.ID}: {monster.DisplayName}");
