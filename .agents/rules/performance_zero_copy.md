# Regras de Performance e Interoperabilidade Zero-Copy

1. **Alinhamento de Memória Arrow**:
   - Toda estrutura colunar deve estar em conformidade com o padrão Apache Arrow em buffers alinhados a 64 bytes para permitir vetorização SIMD (AVX-512, ARM Neon).
   - O processamento de lotes deve adotar chunking colunar (`RecordBatch`) para evitar estouro de memória em datasets de dezenas de gigabytes (ex: SIH, microdados censitários).

2. **Travessia Cross-Language**:
   - A transferência de dados do núcleo Rust para Python (`brhealth-python`), C++20 (`brhealth-ffi`) e JVM (`brhealth-jni`) deve ser efetuada via **Arrow C Data Interface** (`FFI_ArrowArray` e `FFI_ArrowSchema`).
   - Para integração direta com tensores em frameworks de aprendizado de máquina (PyTorch, JAX), utilizar exportação **DLPack**.
   - É estritamente proibida a conversão intermediária para representações textuais ou não contíguas (JSON, CSV, listas Python).

3. **Concorrência e Paralelismo Seguro**:
   - Descompressão e parsing de formatos brutos (.dbc, .dbf) devem utilizar paralelismo via `rayon`.
   - I/O de rede (FTP assíncrono do DATASUS, APIs REST) deve utilizar o runtime `tokio`.
   - Nunca bloquear threads do runtime Tokio com tarefas de computação intensiva (CPU-bound); utilizar `tokio::task::spawn_blocking` ou despachar para o pool Rayon.
