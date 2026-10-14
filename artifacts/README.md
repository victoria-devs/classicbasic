# Reverse Engineering Artifacts & Input Data

Este diretório contém os artefatos de entrada e arquivos de suporte utilizados para a análise e engenharia reversa do runtime **`MSVBVM60.DLL`**.

---

## 📦 Conteúdo do Diretório

| Arquivo | Tamanho | Descrição |
| :--- | :--- | :--- |
| **`msvbvm60.dll`** | ~1.38 MB | Binário original extraído de uma instalação do Windows 2000 Pro SP4 (Versão 6.0.96.90, "Visual Basic Virtual Machine", compilada em 2002-09-03). Utilizado para desmontagem, análise de cabeçalhos e validação de offsets. |
| **`msvbvm60.txt`** | 27 KB | Dump de exports gerado com `dumpbin /exports msvbvm60.dll`. Lista completa dos 635 exports nomeados (ordinais 100 a 2010), hints e RVAs de origem. |
| **`transfer.img`** | 128 MB | Imagem de disco bruto (raw image) montável no emulador UTM/QEMU x86 para transferir arquivos entre o host (macOS Apple Silicon) e a máquina virtual (Windows 2000). |
| **`REFERENCE.md`** | 277 B | Registro das especificações do ambiente de teste e versões de software (Windows 2000 SP4, VB6 v8176/6.0.8169). |

### 📂 `reports/`

- **`rea-doctor.txt`**: Relatório de auditoria e diagnóstico do ambiente de engenharia reversa (Node, host, Hopper, etc.).
- **`package-lock.json`**: Lockfile gerado pelas ferramentas de auditoria e ambiente.
