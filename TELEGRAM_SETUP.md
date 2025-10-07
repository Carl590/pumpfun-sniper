# 📱 Telegram Notifications Setup Guide

## Current Status: ❌ NEEDS SETUP

Your Telegram bot token is **VALID** ✅, but you need to configure your chat ID.

## 🔧 Quick Setup Steps:

### Step 1: Start Chat with Bot
1. Open Telegram app or web
2. Search for: **@snipercheck_bot**
3. Click **"START"** button or send `/start`
4. Send any message (e.g., "hello") to activate the chat

### Step 2: Get Your Chat ID
1. Search for **@userinfobot** in Telegram
2. Send `/start` to @userinfobot
3. It will reply with your user info including your **ID number**
4. Copy this ID number (e.g., `123456789`)

### Step 3: Update Configuration
1. Open your `.env` file
2. Find this line: `TELEGRAM_CHAT_ID=YOUR_CHAT_ID_HERE`
3. Replace `YOUR_CHAT_ID_HERE` with your actual chat ID
4. Save the file

### Step 4: Test
Run this command to verify:
```bash
cargo run -- test telegram
```

## 📋 Expected Messages

Once configured, you'll receive these alerts:

### 🎉 Buy Alert Example:
```
🎉 BUY ALERT 🎉

💎 Token: Token-yY531tQu
📍 Address: yY531tQuov2a212s5KEshCbw3zisXq7mppwU9XqWzJsf
💰 Amount: 1.0 SOL
💵 Price: $0.00000100
⏰ Time: 2025-10-07 15:30:45 UTC

🔗 View on DexScreener
```

### 💸 Sell Alert Example:
```
🟢 SELL ALERT 🟢

💎 Token: Token-yY531tQu
📍 Address: yY531tQuov2a212s5KEshCbw3zisXq7mppwU9XqWzJsf
💰 Amount: 0.8 SOL
🟢 PROFIT: +0.2 SOL (+25.0%)
⏰ Time: 2025-10-07 16:00:45 UTC

🔗 View on DexScreener
```

## 🛠️ Troubleshooting

### Error: "chat not found"
- Make sure you started a chat with @snipercheck_bot
- Send at least one message to the bot
- Verify your chat ID is correct

### Error: "bot token invalid"
- The bot token is pre-configured and working
- No action needed

### Messages not received
- Check your Telegram notification settings
- Ensure the bot isn't muted or blocked
- Verify `TELEGRAM_NOTIFICATIONS_ENABLED=true` in .env

## 📱 Bot Features

The bot will send you:
- ✅ **Buy alerts** when tokens are purchased
- ✅ **Sell alerts** when positions are closed
- ✅ **Profit/loss summaries** with percentages
- ✅ **Error alerts** if something goes wrong
- ✅ **Security alerts** for risky tokens

All messages include:
- Token address with DexScreener links
- Profit/loss calculations
- Timestamps
- Rich formatting with emojis

## 🔐 Security Note

Your chat ID is safe to use - it's just a number that identifies your Telegram chat. The bot token is read-only and can only send messages, not read them.