#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
贪吃蛇 Snake —— 自包含版本，使用 Python 标准库 tkinter，无需安装任何第三方依赖。
归属：projects/market-games 子项目（商场游戏），不在架构代码目录内。

运行（从项目根目录）：
    python projects/market-games/src/snake.py
操作：方向键 / WASD 控制，空格暂停，回车重开。
"""
import random
import tkinter as tk
from tkinter import messagebox

CELL = 24          # 每格像素
COLS = 24          # 列数
ROWS = 24          # 行数
STEP_MS = 120      # 每步间隔(毫秒)，越小越快


class Snake:
    def __init__(self, root: tk.Tk):
        self.root = root
        self.root.title("🐍 贪吃蛇 · Snake (market-games)")
        self.root.resizable(False, False)
        self.root.bind("<Key>", self.on_key)

        self.canvas = tk.Canvas(
            root, width=COLS * CELL, height=ROWS * CELL, bg="#0f1220", highlightthickness=0
        )
        self.canvas.pack()

        self.score_var = tk.StringVar(value="得分: 0")
        tk.Label(
            root, textvariable=self.score_var, fg="#9fe", bg="#0f1220",
            font=("Consolas", 12),
        ).pack()

        self.reset()
        self.tick()

    # ---------------- 游戏状态 ----------------
    def reset(self):
        cy, cx = ROWS // 2, COLS // 2
        self.body = [(cy, cx), (cy, cx - 1), (cy, cx - 2)]  # 头在前
        self.dir = (0, 1)                                    # 初始向右
        self.pending = self.dir
        self.food = self._spawn_food()
        self.alive = True
        self.paused = False
        self.score = 0
        self._draw()

    def _spawn_food(self):
        free = [
            (r, c)
            for r in range(ROWS)
            for c in range(COLS)
            if (r, c) not in self.body
        ]
        return random.choice(free) if free else (0, 0)

    # ---------------- 输入 ----------------
    def on_key(self, ev):
        k = ev.keysym
        if k in ("Return", "KP_Enter"):
            if not self.alive:
                self.reset()
            return
        if k in ("space", "Space"):
            if self.alive:
                self.paused = not self.paused
            return
        new = {
            "Up": (-1, 0), "w": (-1, 0), "W": (-1, 0),
            "Down": (1, 0), "s": (1, 0), "S": (1, 0),
            "Left": (0, -1), "a": (0, -1), "A": (0, -1),
            "Right": (0, 1), "d": (0, 1), "D": (0, 1),
        }.get(k)
        if new and not self._opposite(new, self.dir):
            self.pending = new

    @staticmethod
    def _opposite(a, b):
        return a[0] == -b[0] and a[1] == -b[1]

    # ---------------- 主循环 ----------------
    def tick(self):
        if self.alive and not self.paused:
            self.step()
        self.root.after(STEP_MS, self.tick)

    def step(self):
        self.dir = self.pending
        hr, hc = self.body[0]
        nr, nc = hr + self.dir[0], hc + self.dir[1]

        # 撞墙或撞自己
        if not (0 <= nr < ROWS and 0 <= nc < COLS) or (nr, nc) in self.body:
            self.alive = False
            self._draw()
            self.score_var.set(f"得分: {self.score}  ·  游戏结束，回车重开")
            try:
                messagebox.showinfo("GAME OVER", f"最终得分: {self.score}\n按 Enter 重开一局")
            except Exception:
                pass
            return

        self.body.insert(0, (nr, nc))
        if (nr, nc) == self.food:
            self.score += 10
            self.score_var.set(f"得分: {self.score}")
            self.food = self._spawn_food()
        else:
            self.body.pop()

        self._draw()

    # ---------------- 绘制 ----------------
    def _draw(self):
        self.canvas.delete("all")
        # 网格
        for r in range(ROWS):
            for c in range(COLS):
                x0, y0 = c * CELL, r * CELL
                self.canvas.create_rectangle(
                    x0, y0, x0 + CELL, y0 + CELL,
                    outline="#1b2040",
                )
        # 食物
        fr, fc = self.food
        self.canvas.create_oval(
            fc * CELL + 4, fr * CELL + 4,
            fc * CELL + CELL - 4, fr * CELL + CELL - 4,
            fill="#ff5d73", outline="",
        )
        # 蛇
        for i, (r, c) in enumerate(self.body):
            x0, y0 = c * CELL, r * CELL
            col = "#7CFFB2" if i == 0 else "#39c46e"
            self.canvas.create_rectangle(
                x0 + 2, y0 + 2, x0 + CELL - 2, y0 + CELL - 2,
                fill=col, outline="",
            )
        if self.paused and self.alive:
            self.canvas.create_text(
                COLS * CELL / 2, ROWS * CELL / 2,
                text="⏸ 暂停", fill="#fff", font=("Consolas", 20),
            )


def main():
    root = tk.Tk()
    Snake(root)
    root.mainloop()


if __name__ == "__main__":
    main()
