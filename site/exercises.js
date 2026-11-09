// 由 scripts/export_site_exercises.py 从 src-tauri/src/store.rs SEED_JSON 生成——勿手改
// （改动作/关键帧请改种子后重跑脚本，与应用内示范保持同一份数据）
const SPAR_EXERCISES = [
  {
    "id": "pushup",
    "nameZh": "俯卧撑",
    "nameEn": "Push-ups",
    "unit": "reps",
    "goal": 10,
    "cuesZh": "核心收紧，身体成一条直线；下到肘约 90°。",
    "cuesEn": "Keep core tight, body in a line; lower to ~90° elbows.",
    "keyframes": {
      "anchor": "hand",
      "frames": [
        {
          "p": [
            90,
            0,
            0,
            -90,
            0,
            0
          ],
          "hold": 500
        },
        {
          "p": [
            90,
            -8,
            95,
            -90,
            0,
            0
          ],
          "hold": 400
        }
      ]
    }
  },
  {
    "id": "knee_pushup",
    "nameZh": "跪姿俯卧撑",
    "nameEn": "Knee Push-ups",
    "unit": "reps",
    "goal": 12,
    "cuesZh": "膝盖着地降低难度，从头到膝仍成一条直线。",
    "cuesEn": "Knees down for an easier scale; head-to-knees still a straight line.",
    "keyframes": {
      "anchor": "hand",
      "frames": [
        {
          "p": [
            70,
            -5,
            0,
            -66,
            24,
            0
          ],
          "hold": 500
        },
        {
          "p": [
            70,
            -15,
            95,
            -66,
            24,
            0
          ],
          "hold": 400
        }
      ]
    }
  },
  {
    "id": "squat",
    "nameZh": "深蹲",
    "nameEn": "Squats",
    "unit": "reps",
    "goal": 15,
    "cuesZh": "臀部向后坐，大腿至少与地面平行，膝盖对脚尖。",
    "cuesEn": "Hips back, thighs at least parallel; knees track over toes.",
    "keyframes": {
      "anchor": "feet",
      "frames": [
        {
          "p": [
            5,
            10,
            0,
            5,
            5,
            0
          ],
          "hold": 450
        },
        {
          "p": [
            30,
            85,
            0,
            100,
            105,
            0
          ],
          "hold": 400
        }
      ]
    }
  },
  {
    "id": "calf_raise",
    "nameZh": "提踵",
    "nameEn": "Calf Raises",
    "unit": "reps",
    "goal": 20,
    "cuesZh": "脚跟尽量抬高，顶端停一拍再缓缓落下。",
    "cuesEn": "Rise high onto the balls of your feet; pause, then lower slowly.",
    "keyframes": {
      "anchor": "feet",
      "frames": [
        {
          "p": [
            3,
            8,
            0,
            3,
            3,
            0
          ],
          "hold": 400
        },
        {
          "p": [
            3,
            14,
            0,
            3,
            8,
            -9
          ],
          "hold": 350
        }
      ]
    }
  },
  {
    "id": "wall_sit",
    "nameZh": "靠墙静蹲",
    "nameEn": "Wall Sit",
    "unit": "seconds",
    "goal": 45,
    "cuesZh": "背贴墙，大腿与地面平行，膝盖不过脚尖。",
    "cuesEn": "Back flat on the wall, thighs parallel to the floor.",
    "keyframes": {
      "anchor": "feet",
      "frames": [
        {
          "p": [
            0,
            25,
            0,
            90,
            92,
            0
          ],
          "hold": 900
        },
        {
          "p": [
            0,
            27,
            0,
            91,
            93,
            -1
          ],
          "hold": 900
        }
      ]
    }
  },
  {
    "id": "desk_dip",
    "nameZh": "桌边臂屈伸",
    "nameEn": "Desk Dips",
    "unit": "reps",
    "goal": 12,
    "cuesZh": "手撑桌沿、背对桌子，肘向后弯沉身体。",
    "cuesEn": "Hands on desk edge, back to the desk; bend elbows to sink down.",
    "keyframes": {
      "anchor": "hand",
      "frames": [
        {
          "p": [
            -10,
            -115,
            0,
            70,
            -25,
            0
          ],
          "hold": 450
        },
        {
          "p": [
            -12,
            -170,
            43,
            72,
            -25,
            0
          ],
          "hold": 400
        }
      ]
    }
  },
  {
    "id": "plank",
    "nameZh": "平板支撑",
    "nameEn": "Plank",
    "unit": "seconds",
    "goal": 40,
    "cuesZh": "肘撑地正下方，从头到脚跟一条直线，别塌腰。",
    "cuesEn": "Elbows under shoulders; head-to-heels one straight line.",
    "keyframes": {
      "anchor": "hand",
      "frames": [
        {
          "p": [
            90,
            0,
            90,
            -90,
            0,
            0
          ],
          "hold": 800
        },
        {
          "p": [
            90,
            0,
            92,
            -90,
            2,
            -1
          ],
          "hold": 800
        }
      ]
    }
  },
  {
    "id": "jumping_jack",
    "nameZh": "开合跳",
    "nameEn": "Jumping Jacks",
    "unit": "reps",
    "goal": 30,
    "cuesZh": "手臂过头打开，双脚开合，保持节奏轻落地。",
    "cuesEn": "Arms overhead, feet in and out; stay light on the landing.",
    "keyframes": {
      "anchor": "feet",
      "frames": [
        {
          "p": [
            0,
            12,
            0,
            3,
            3,
            0
          ],
          "hold": 350
        },
        {
          "p": [
            0,
            172,
            5,
            25,
            2,
            -7
          ],
          "hold": 350
        }
      ]
    }
  }
];
