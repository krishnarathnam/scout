#!/usr/bin/env python3
"""Fetch and print balance sheet for Tata Steel (NSE).
   Install: pip install yfinance
   Run:     python scripts/balance_sheet_tatasteel.py
"""
import yfinance as yf

symbol = "TATASTEEL.NS"
ticker = yf.Ticker(symbol)
bs = ticker.income_stmt

if bs is None or bs.empty:
    print("No balance sheet data returned.")
else:
    print(f"Balance Sheet — {symbol}\n")
    print(bs.to_string())
