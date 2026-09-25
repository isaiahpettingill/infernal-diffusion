# Bundled BERT miniature

- `config.json`, `model.safetensors`, and `vocab.txt` are from [Google's two-layer BERT miniature](https://huggingface.co/google/bert_uncased_L-2_H-128_A-2) at revision `30b0a37ccaaa32f332884b96992754e246e48c5f`.
- `tokenizer.json` is from [Google BERT base uncased](https://huggingface.co/google-bert/bert-base-uncased) at revision `86b5e0934494bd15c9632b12f734a8a67f723594`.
- Both upstream model repositories identify their license as Apache-2.0. The license text is in `LICENSE-APACHE-2.0.txt`.
- `model.safetensors` SHA-256: `7fb69ad9f6866d8983183c930e33828f326470bf6ad8bbb2ad4ed957a92e9414`.
- The tokenizer WordPiece vocabulary IDs were compared against all 30,522 entries in `vocab.txt` and match exactly.

The embedded model is a pretrained encoder. This repository does not include a fine-tuned monster classification head. The generator uses prompt vocabulary extraction plus BERT prototype similarity; an externally trained `semantic_head.json` can be supplied with a custom model directory.
