# mryt_protocol

# 🏦 Multi-Chain Real Yield Token (MRYT)

📌 **MRYT (Multi-Chain Real Yield Token)** is a Solana-based **yield-bearing token** that distributes DeFi revenue to stakers. This protocol allows users to stake **LP tokens** to earn real yield, not inflationary emissions.

## 🚀 **Features**
✅ **Stake LP Tokens** – Users deposit LP tokens into the vault and receive MRYT in return.  
✅ **Real Yield Accrual** – The protocol generates yield from DeFi activities.  
✅ **Auto-Compounding** – A portion of yield is automatically reinvested to maximize long-term returns.  
✅ **Sybil-Resistant** – Prevents large withdrawals by enforcing caps and vesting.  
✅ **Flash Loan-Proof** – New deposits are locked for **7 days** to prevent exploits.  
✅ **Dynamic APY Calculation** – The system dynamically calculates the **Annual Percentage Yield (APY)** based on accrued rewards.

---

## 📜 **Smart Contract Overview**
The program is written in **Rust using the Anchor framework** and is tested in **Solana Playground**.

---

### 🔹 **Program Instructions**
| Instruction          | Description |
|----------------------|-------------|
| `initialize`        | Initializes the protocol and creates the **MRYT mint**. |
| `deposit`          | Users deposit **LP tokens** and mint **MRYT tokens** (1:1 ratio). |
| `withdraw`         | Users burn **MRYT tokens** to redeem their staked **LP tokens** (subject to lock-up). |
| `accrue_yield`     | Simulates yield accrual (e.g., 1% of total staked is added as yield). |
| `auto_compound_yield` | Automatically reinvests **50% of accrued yield** to boost earnings. |
| `calculate_apy`    | Computes the **APY (Annual Percentage Yield)** based on real earnings. |

---
