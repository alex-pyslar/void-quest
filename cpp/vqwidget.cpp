// vqwidget.cpp — VoidQuest 3D MMORPG client renderer.
//
// Architecture:
//   • VqWidget3D owns a rust::Box<GameApp> (all game logic in Rust).
//   • paintGL()  → renderWorld3D() / renderWorld2D() + QPainter HUD overlay.
//   • keyPressEvent() → game_->on_key() → Cmd (Quit closes window).
//   • 30 FPS QTimer → game_->tick() → update().
//   • Mouse events → drag/resize inventory & equipment windows.
//
// Camera modes (F5 to cycle):
//   0 = TopDown    — classic isometric look-down
//   1 = ThirdPerson— over-shoulder following player facing
//   2 = FirstPerson— eye-level, 70° FOV, looking where player faces
//   3 = 2D-pixel   — top-down QPainter colored tiles (no OpenGL)
//
// Entity rendering:
//   0 (self)   — 3D box with pulse glow
//   1 (player) — Doom-style billboard sprite
//   2 (monster)— Doom-style billboard sprite
//   3 (item)   — small spinning box

#include "vqwidget.h"
#include "void-quest/src/client/bridge.rs.h"

#include <QApplication>
#include <QOpenGLWidget>
#include <QOpenGLFunctions_3_3_Core>
#include <QOpenGLShaderProgram>
#include <QOpenGLBuffer>
#include <QOpenGLVertexArrayObject>
#include <QPainter>
#include <QFont>
#include <QFontMetrics>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QTimer>
#include <QLinearGradient>
#include <QSurfaceFormat>
#include <QMatrix4x4>
#include <QVector3D>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>
#include <cmath>
#include <vector>
#include <string>
#include <algorithm>

namespace vq {

// ── Tile visual style ─────────────────────────────────────────────────────────

struct TileStyle {
    float r, g, b;   // base color (0-1)
    float height;    // extrusion height (0 = flat)
    bool  extruded;  // true = render side faces
};

// 2D palette (indexed by TileKind u8)
struct TileColor2D { int r, g, b; };

static const TileStyle TILE_STYLES[13] = {
    {0.22f, 0.52f, 0.10f, 0.00f, false}, // 0  Grass
    {0.42f, 0.42f, 0.46f, 0.72f, true }, // 1  Wall
    {0.08f, 0.36f, 0.05f, 0.65f, true }, // 2  Tree
    {0.10f, 0.20f, 0.62f, 0.00f, false}, // 3  Water
    {0.30f, 0.30f, 0.33f, 0.00f, false}, // 4  Floor
    {0.50f, 0.46f, 0.30f, 0.00f, false}, // 5  Road
    {0.80f, 0.70f, 0.40f, 0.00f, false}, // 6  Sand
    {0.90f, 0.28f, 0.02f, 0.00f, false}, // 7  Lava
    {0.70f, 0.86f, 0.90f, 0.00f, false}, // 8  Ice
    {0.50f, 0.50f, 0.54f, 0.80f, true }, // 9  Pillar
    {0.40f, 0.20f, 0.10f, 0.32f, true }, // 10 Bramble
    {0.46f, 0.40f, 0.33f, 0.00f, false}, // 11 Ruins
    {0.36f, 0.28f, 0.10f, 0.00f, false}, // 12 Mud
};

static const TileColor2D TILE_2D[13] = {
    { 55, 133,  26}, // 0  Grass
    {107, 107, 117}, // 1  Wall
    { 20,  92,  13}, // 2  Tree
    { 26,  51, 159}, // 3  Water
    { 77,  77,  84}, // 4  Floor
    {128, 118,  77}, // 5  Road
    {204, 179, 102}, // 6  Sand
    {230,  71,   5}, // 7  Lava
    {179, 220, 230}, // 8  Ice
    {128, 128, 138}, // 9  Pillar
    {102,  51,  26}, // 10 Bramble
    {117, 102,  84}, // 11 Ruins
    { 92,  71,  26}, // 12 Mud
};

// ── Mesh helpers ──────────────────────────────────────────────────────────────

struct Vertex { float x,y,z, r,g,b, nx,ny,nz; };

static void pushQuad(std::vector<Vertex>& v,
                     float x0,float y0,float z0,
                     float x1,float y1,float z1,
                     float x2,float y2,float z2,
                     float x3,float y3,float z3,
                     float r, float g, float b,
                     float nx,float ny,float nz)
{
    v.push_back({x0,y0,z0, r,g,b, nx,ny,nz});
    v.push_back({x1,y1,z1, r,g,b, nx,ny,nz});
    v.push_back({x2,y2,z2, r,g,b, nx,ny,nz});
    v.push_back({x0,y0,z0, r,g,b, nx,ny,nz});
    v.push_back({x2,y2,z2, r,g,b, nx,ny,nz});
    v.push_back({x3,y3,z3, r,g,b, nx,ny,nz});
}

static void pushTile(std::vector<Vertex>& verts, float tx, float tz,
                     const TileStyle& s, float waveY = 0.f, float glow = 1.f)
{
    float h   = s.height;
    float x0  = tx, x1 = tx + 1.f;
    float z0  = tz, z1 = tz + 1.f;
    float topY = h + waveY;
    float tr = s.r * glow, tg = s.g * glow, tb = s.b * glow;

    // Top face
    pushQuad(verts,
        x0,topY,z0,  x1,topY,z0,  x1,topY,z1,  x0,topY,z1,
        tr, tg, tb,  0,1,0);

    if (s.extruded && h > 0.f) {
        float dr = tr * 0.70f, dg = tg * 0.70f, db = tb * 0.70f;
        float er = tr * 0.82f, eg = tg * 0.82f, eb = tb * 0.82f;
        pushQuad(verts, x0,0,z1, x1,0,z1, x1,h,z1, x0,h,z1, dr,dg,db, 0,0,1);
        pushQuad(verts, x1,0,z0, x0,0,z0, x0,h,z0, x1,h,z0, dr*.85f,dg*.85f,db*.85f, 0,0,-1);
        pushQuad(verts, x1,0,z0, x1,0,z1, x1,h,z1, x1,h,z0, er,eg,eb, 1,0,0);
        pushQuad(verts, x0,0,z1, x0,0,z0, x0,h,z0, x0,h,z1, er*.85f,eg*.85f,eb*.85f, -1,0,0);
    }
}

static void pushEntityBox(std::vector<Vertex>& verts,
                          float tx, float tz,
                          float r, float g, float b,
                          float w, float h)
{
    float margin = (1.f - w) * .5f;
    float x0 = tx + margin, x1 = tx + 1.f - margin;
    float z0 = tz + margin, z1 = tz + 1.f - margin;
    pushQuad(verts, x0,h,z0, x1,h,z0, x1,h,z1, x0,h,z1, r,g,b, 0,1,0);
    pushQuad(verts, x0,0,z1, x1,0,z1, x1,h,z1, x0,h,z1, r*.68f,g*.68f,b*.68f, 0,0,1);
    pushQuad(verts, x1,0,z0, x0,0,z0, x0,h,z0, x1,h,z0, r*.60f,g*.60f,b*.60f, 0,0,-1);
    pushQuad(verts, x1,0,z0, x1,0,z1, x1,h,z1, x1,h,z0, r*.82f,g*.82f,b*.82f, 1,0,0);
    pushQuad(verts, x0,0,z1, x0,0,z0, x0,h,z0, x0,h,z1, r*.75f,g*.75f,b*.75f, -1,0,0);
}

// Doom-style billboard: always faces camera, uses camera-right vector
static void pushBillboard(std::vector<Vertex>& verts,
                           float tx, float tz,
                           float r, float g, float b,
                           float w, float h,
                           const QVector3D& camRight)
{
    float cx = tx + 0.5f, cz = tz + 0.5f;
    float hw = w * 0.5f;
    float hh = h;

    float rx = camRight.x() * hw;
    float rz = camRight.z() * hw;

    // Four corners: BL, BR, TR, TL
    float blx = cx - rx, blz = cz - rz, bly = 0.f;
    float brx = cx + rx, brz = cz + rz, bry = 0.f;
    float trx = cx + rx, trz = cz + rz, try_ = hh;
    float tlx = cx - rx, tlz = cz - rz, tly = hh;

    float topR = r, topG = g, topB = b;
    float botR = r * 0.6f, botG = g * 0.6f, botB = b * 0.6f;

    // Billboard face normal = cross(right, worldUp) = (-rz, 0, rx)
    // This makes the sprite lit from the sun like a vertical wall facing camera.
    float nx = -camRight.z(), ny = 0.f, nz = camRight.x();

    // Triangle 1: BL, BR, TR
    verts.push_back({blx, bly, blz, botR,botG,botB, nx,ny,nz});
    verts.push_back({brx, bry, brz, botR,botG,botB, nx,ny,nz});
    verts.push_back({trx, try_, trz, topR,topG,topB, nx,ny,nz});
    // Triangle 2: BL, TR, TL
    verts.push_back({blx, bly, blz, botR,botG,botB, nx,ny,nz});
    verts.push_back({trx, try_, trz, topR,topG,topB, nx,ny,nz});
    verts.push_back({tlx, tly, tlz, topR,topG,topB, nx,ny,nz});
}

// ── QPainter helpers ──────────────────────────────────────────────────────────

static void panel(QPainter& p, int x, int y, int w, int h,
                  QColor border, QColor bg,
                  const QString& title = {})
{
    p.fillRect(x, y, w, h, bg);
    p.setPen(QPen(border, 2));
    p.drawRect(x, y, w-1, h-1);
    if (!title.isEmpty()) {
        p.setFont(QFont("Consolas", 10, QFont::Bold));
        p.setPen(QColor(0, 220, 255));
        QRect tr(x+8, y+2, w-16, 18);
        p.fillRect(tr, bg);
        p.drawText(tr, Qt::AlignVCenter, title);
    }
}

static void hpBar(QPainter& p, int x, int y, int w, int h,
                  const QString& label, int val, int maxVal, QColor barCol)
{
    p.fillRect(x, y, w, h, QColor(18, 18, 38));
    p.setPen(QColor(60, 60, 90));
    p.drawRect(x, y, w-1, h-1);
    if (maxVal > 0) {
        int bw = (int)((float)val / maxVal * (w-2));
        bw = std::max(0, std::min(bw, w-2));
        p.fillRect(x+1, y+1, bw, h-2, barCol);
    }
    p.setFont(QFont("Consolas", 8, QFont::Bold));
    p.setPen(Qt::white);
    p.drawText(x+3, y+h-3, QString("%1 %2/%3").arg(label).arg(val).arg(maxVal));
}

static QString rs(const rust::String& s) {
    return QString::fromStdString(std::string(s));
}

// ── Camera setup ──────────────────────────────────────────────────────────────

struct CamSetup {
    QVector3D eye, target, up;
    float     fov;
    QVector3D right; // precomputed camera-right for billboards
};

static CamSetup makeCamera(uint8_t mode, float cx, float cz,
                            int face_dx, int face_dy)
{
    CamSetup cam;
    cam.up = QVector3D(0, 1, 0);

    switch (mode) {
    default:
    case 0: // TopDown
        cam.eye    = QVector3D(cx - 0.5f, 9.5f, cz + 8.0f);
        cam.target = QVector3D(cx, 0.f, cz);
        cam.fov    = 42.f;
        break;
    case 1: // ThirdPerson — over shoulder
        {
            float fdx = -(float)face_dx * 4.5f;
            float fdz = -(float)face_dy * 4.5f;
            cam.eye    = QVector3D(cx + fdx, 3.2f, cz + fdz + 0.3f);
            cam.target = QVector3D(cx + (float)face_dx * 3.f, 0.5f,
                                   cz + (float)face_dy * 3.f);
            cam.fov    = 60.f;
        }
        break;
    case 2: // FirstPerson — eye level
        {
            // Avoid zero-length forward vector
            float fdx = (face_dx == 0 && face_dy == 0) ? 0.f : (float)face_dx;
            float fdz = (face_dx == 0 && face_dy == 0) ? 1.f : (float)face_dy;
            cam.eye    = QVector3D(cx + 0.5f, 0.72f, cz + 0.5f);
            cam.target = QVector3D(cx + 0.5f + fdx * 10.f, 0.72f,
                                   cz + 0.5f + fdz * 10.f);
            cam.fov    = 70.f;
        }
        break;
    case 3: // 2D-pixel: unused by OpenGL path
        cam.eye    = QVector3D(cx, 20.f, cz);
        cam.target = QVector3D(cx, 0.f, cz);
        cam.up     = QVector3D(0.f, 0.f, -1.f);
        cam.fov    = 42.f;
        break;
    }

    // Compute right vector for billboard alignment
    QVector3D fwd = (cam.target - cam.eye).normalized();
    cam.right = QVector3D::crossProduct(fwd, cam.up).normalized();
    return cam;
}

// ── Draggable window state ─────────────────────────────────────────────────────

struct DragWin {
    int x, y, w, h;
};

// ── VqWidget3D ────────────────────────────────────────────────────────────────

class VqWidget3D : public QOpenGLWidget, protected QOpenGLFunctions_3_3_Core {
public:
    explicit VqWidget3D(rust::Box<GameApp> game, QWidget* parent = nullptr)
        : QOpenGLWidget(parent), game_(std::move(game))
    {
        setWindowTitle("VoidQuest — 3D MMORPG");
        setMinimumSize(1024, 640);
        setFocusPolicy(Qt::StrongFocus);
        setAttribute(Qt::WA_InputMethodEnabled, false);
        setMouseTracking(true);

        auto* timer = new QTimer(this);
        QObject::connect(timer, &QTimer::timeout, this, [this]() {
            game_->tick();
            update();
        });
        timer->start(33);
    }

protected:
    // ── OpenGL lifecycle ──────────────────────────────────────────────────────

    void initializeGL() override {
        initializeOpenGLFunctions();
        glEnable(GL_DEPTH_TEST);
        // No face culling — needed for billboard sprites
        glClearColor(0.03f, 0.03f, 0.07f, 1.f);

        prog_ = new QOpenGLShaderProgram(this);
        prog_->addShaderFromSourceCode(QOpenGLShader::Vertex, R"(
#version 330 core
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aCol;
layout(location=2) in vec3 aNrm;
uniform mat4 uMVP;
uniform vec3 uSun;
uniform vec3 uFill;
uniform float uAmb;
out vec3 vCol;
void main(){
    gl_Position = uMVP * vec4(aPos,1.0);
    float sun  = max(dot(normalize(aNrm), normalize(uSun)),  0.0);
    float fill = max(dot(normalize(aNrm), normalize(uFill)), 0.0) * 0.25;
    vCol = aCol * (uAmb + sun * 0.65 + fill);
    vCol = clamp(vCol, 0.0, 1.0);
}
)");
        prog_->addShaderFromSourceCode(QOpenGLShader::Fragment, R"(
#version 330 core
in vec3 vCol;
out vec4 frag;
void main(){ frag = vec4(vCol,1.0); }
)");
        prog_->link();

        vao_.create();
        vbo_.create();
        vbo_.setUsagePattern(QOpenGLBuffer::DynamicDraw);
    }

    void resizeGL(int w, int h) override {
        W_ = w; H_ = h;
        int panelPx  = w * 72 / 100;
        int gamePxH  = h - TOP_BAR - LOG_H;
        game_->on_resize(std::max(8, panelPx / 52), std::max(6, gamePxH / 52));
        // Keep windows within bounds
        clampWin(invWin_);
        clampWin(eqWin_);
    }

    void paintGL() override {
        HudData hud = game_->get_hud();

        glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

        if (hud.screen == 4 && hud.camera_mode != 3) {
            renderWorld3D(hud);
        }

        QPainter p(this);
        p.setRenderHint(QPainter::Antialiasing);
        p.setRenderHint(QPainter::TextAntialiasing);

        switch (hud.screen) {
            case 0: drawMainMenu(p, hud);   break;
            case 1: drawConnect(p, hud);    break;
            case 2: drawLogin(p, hud);      break;
            case 3: drawCharCreate(p, hud); break;
            case 4:
                if (hud.camera_mode == 3) renderWorld2D(p, hud);
                drawPlayingHUD(p, hud);
                break;
        }
        if (!hud.error_msg.empty())
            drawError(p, rs(hud.error_msg));

        p.end();
    }

    void keyPressEvent(QKeyEvent* ev) override {
        std::string txt = ev->text().toStdString();
        rust::Str rs_txt(txt.data(), txt.size());
        Cmd cmd = game_->on_key(ev->key(), (uint32_t)ev->modifiers(), rs_txt);
        if (cmd.kind == CmdKind::Quit) close();
    }

    void mousePressEvent(QMouseEvent* ev) override {
        if (ev->button() != Qt::LeftButton) return;
        int mx = ev->pos().x(), my = ev->pos().y();
        HudData hud = game_->get_hud();
        if (hud.screen != 4) return;

        // Check equip window first (drawn on top)
        if (hud.equip_open && hitTitle(mx, my, eqWin_)) {
            dragWhich_ = 2; dragOX_ = mx - eqWin_.x; dragOY_ = my - eqWin_.y;
            dragResize_ = false;
        } else if (hud.equip_open && hitResize(mx, my, eqWin_)) {
            dragWhich_ = 2; dragOX_ = mx; dragOY_ = my;
            dragResize_ = true;
        } else if (hud.inv_open && hitTitle(mx, my, invWin_)) {
            dragWhich_ = 1; dragOX_ = mx - invWin_.x; dragOY_ = my - invWin_.y;
            dragResize_ = false;
        } else if (hud.inv_open && hitResize(mx, my, invWin_)) {
            dragWhich_ = 1; dragOX_ = mx; dragOY_ = my;
            dragResize_ = true;
        }
    }

    void mouseMoveEvent(QMouseEvent* ev) override {
        if (dragWhich_ == 0) return;
        int mx = ev->pos().x(), my = ev->pos().y();
        DragWin& dw = (dragWhich_ == 1) ? invWin_ : eqWin_;
        if (dragResize_) {
            dw.w = std::max(240, dw.w + (mx - dragOX_));
            dw.h = std::max(120, dw.h + (my - dragOY_));
            dragOX_ = mx; dragOY_ = my;
        } else {
            dw.x = mx - dragOX_;
            dw.y = my - dragOY_;
        }
        clampWin(dw);
        update();
    }

    void mouseReleaseEvent(QMouseEvent* ev) override {
        if (ev->button() == Qt::LeftButton) dragWhich_ = 0;
    }

private:
    rust::Box<GameApp>        game_;
    QOpenGLShaderProgram*     prog_ = nullptr;
    QOpenGLBuffer             vbo_{ QOpenGLBuffer::VertexBuffer };
    QOpenGLVertexArrayObject  vao_;
    int W_ = 1280, H_ = 800;

    static constexpr int TOP_BAR = 34;
    static constexpr int RIGHT_W = 290;
    static constexpr int LOG_H   = 140;

    // Draggable windows
    DragWin invWin_  {120, 120, 410, 380};
    DragWin eqWin_   {560, 120, 410, 220};
    int     dragWhich_   = 0;   // 0=none 1=inv 2=equip
    bool    dragResize_  = false;
    int     dragOX_ = 0, dragOY_ = 0;

    static bool hitTitle(int mx, int my, const DragWin& w) {
        return mx >= w.x && mx < w.x + w.w && my >= w.y && my < w.y + 22;
    }
    static bool hitResize(int mx, int my, const DragWin& w) {
        return mx >= w.x + w.w - 16 && mx < w.x + w.w
            && my >= w.y + w.h - 16 && my < w.y + w.h;
    }
    void clampWin(DragWin& w) {
        w.x = std::max(0, std::min(w.x, W_ - w.w));
        w.y = std::max(TOP_BAR, std::min(w.y, H_ - w.h));
    }

    // ── 3D world rendering ────────────────────────────────────────────────────

    void renderWorld3D(const HudData& hud) {
        MapInfo info = game_->get_map_info();
        rust::Vec<EntityInfo> entities = game_->get_entities();

        float cx = (float)info.player_x + 0.5f;
        float cz = (float)info.player_y + 0.5f;
        CamSetup cam = makeCamera(hud.camera_mode, cx, cz,
                                  hud.player_face_dx, hud.player_face_dy);

        // Sky color depends on camera mode
        if (hud.camera_mode == 2) {
            glClearColor(0.38f, 0.52f, 0.72f, 1.f); // first-person: daytime sky
        } else {
            glClearColor(0.04f, 0.06f, 0.14f, 1.f);
        }
        glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

        std::vector<Vertex> verts;
        verts.reserve(100000);

        // Tiles — extend render range for first-person
        int extR = (hud.camera_mode == 2) ? 3 : 1;
        float tick = (float)hud.anim_tick;
        float wave  = std::sin(tick * 0.08f) * 0.04f;
        float lavaP = 0.85f + 0.15f * std::sin(tick * 0.12f); // lava pulse

        for (int ty = info.cam_y - extR; ty <= info.cam_y + info.view_h + extR; ++ty) {
            for (int tx = info.cam_x - extR; tx <= info.cam_x + info.view_w + extR; ++tx) {
                uint8_t kind = game_->get_tile(tx, ty);
                if (kind >= 13) kind = 0;
                const TileStyle& s = TILE_STYLES[kind];
                float wY   = (kind == 3 || kind == 7) ? wave - 0.04f : 0.f;
                float glow = (kind == 7) ? lavaP : 1.f; // lava glow
                pushTile(verts, (float)tx, (float)ty, s, wY, glow);
            }
        }

        // Entities
        for (const auto& e : entities) {
            float r = e.r / 255.f, g = e.g / 255.f, b = e.b / 255.f;
            switch (e.kind) {
            case 0: { // self — 3D box with pulse
                float pulse = 0.15f * std::sin((float)e.anim * 0.15f);
                pushEntityBox(verts, e.x, e.y,
                              r + pulse*.3f, g + pulse*.3f, b + pulse*.3f,
                              0.48f, 0.92f);
                break;
            }
            case 1: // other player — billboard
                pushBillboard(verts, e.x, e.y, r*.9f, g*.9f, b*.9f,
                              0.72f, 0.84f, cam.right);
                break;
            case 2: // monster — billboard
                pushBillboard(verts, e.x, e.y, r, g, b,
                              0.80f, 0.72f, cam.right);
                break;
            case 3: { // ground item — small spinning box
                float spin = std::sin((float)e.anim * 0.18f) * 0.35f;
                pushEntityBox(verts, (float)e.x + spin*.05f, e.y,
                              r, g, b, 0.32f, 0.14f);
                break;
            }
            }
        }

        if (verts.empty()) return;

        QMatrix4x4 view;
        view.lookAt(cam.eye, cam.target, cam.up);

        QMatrix4x4 proj;
        float aspect = (float)W_ / (float)std::max(1, H_);
        proj.perspective(cam.fov, aspect, 0.1f, 300.f);

        QMatrix4x4 mvp = proj * view;

        vao_.bind();
        vbo_.bind();
        vbo_.allocate(verts.data(), (int)(verts.size() * sizeof(Vertex)));

        prog_->bind();
        prog_->setUniformValue("uMVP",  mvp);
        prog_->setUniformValue("uSun",  QVector3D(-0.45f, 1.0f, -0.4f));
        prog_->setUniformValue("uFill", QVector3D(0.45f, 0.3f, 0.5f));
        prog_->setUniformValue("uAmb",  0.30f);

        int stride = sizeof(Vertex);
        prog_->enableAttributeArray(0);
        prog_->setAttributeBuffer(0, GL_FLOAT, 0,               3, stride);
        prog_->enableAttributeArray(1);
        prog_->setAttributeBuffer(1, GL_FLOAT, 3*sizeof(float),  3, stride);
        prog_->enableAttributeArray(2);
        prog_->setAttributeBuffer(2, GL_FLOAT, 6*sizeof(float),  3, stride);

        glDrawArrays(GL_TRIANGLES, 0, (GLsizei)verts.size());

        prog_->release();
        vbo_.release();
        vao_.release();
    }

    // ── 2D pixel renderer (camera_mode == 3) ─────────────────────────────────

    void renderWorld2D(QPainter& p, const HudData& hud) {
        int gameW = W_ - RIGHT_W;
        int gameH = H_ - TOP_BAR - LOG_H;
        int offY  = TOP_BAR;
        MapInfo info = game_->get_map_info();
        rust::Vec<EntityInfo> entities = game_->get_entities();

        // Fill game area
        p.fillRect(0, offY, gameW, gameH, QColor(6, 6, 16));

        int vw = info.view_w, vh = info.view_h;
        int tileW = std::max(4, std::min(gameW / std::max(1, vw),
                                         gameH / std::max(1, vh)));
        int totalW = tileW * vw, totalH = tileW * vh;
        int startX = (gameW - totalW) / 2;
        int startY = offY + (gameH - totalH) / 2;

        float tick = (float)hud.anim_tick;
        float lavaP = 0.85f + 0.15f * std::sin(tick * 0.12f);

        for (int ty = 0; ty < vh; ++ty) {
            for (int tx = 0; tx < vw; ++tx) {
                int wx = info.cam_x + tx, wy = info.cam_y + ty;
                uint8_t kind = game_->get_tile(wx, wy);
                if (kind >= 13) kind = 0;
                auto& c = TILE_2D[kind];
                float g = (kind == 7) ? lavaP : 1.f;
                p.fillRect(startX + tx * tileW, startY + ty * tileW,
                           tileW - 1, tileW - 1,
                           QColor((int)(c.r*g), (int)(c.g*g), (int)(c.b*g)));
            }
        }

        // Draw entities
        p.setFont(QFont("Consolas", tileW > 12 ? 11 : 8, QFont::Bold));
        for (const auto& e : entities) {
            int ex = e.x - info.cam_x, ey = e.y - info.cam_y;
            if (ex < 0 || ex >= vw || ey < 0 || ey >= vh) continue;
            int px = startX + ex * tileW + 1;
            int py = startY + ey * tileW;
            QColor col(e.r, e.g, e.b);
            QString sym;
            switch (e.kind) {
            case 0: sym = "@"; col = QColor(0, 255, 220); break;  // self
            case 1: sym = "@"; break;                             // other player
            case 2: sym = "!"; break;                             // monster
            case 3: sym = "*"; break;                             // item
            }
            p.fillRect(px, py+1, tileW-2, tileW-2, QColor(0,0,0,100));
            p.setPen(col);
            p.drawText(QRect(px, py, tileW, tileW), Qt::AlignCenter, sym);
        }
    }

    // ── Screen renderers ──────────────────────────────────────────────────────

    void drawMainMenu(QPainter& p, const HudData& hud) {
        QLinearGradient bg(0,0,0,H_);
        bg.setColorAt(0, QColor(4,4,22));
        bg.setColorAt(1, QColor(8,8,36));
        p.fillRect(0, 0, W_, H_, bg);

        // Star field
        for (int i = 0; i < 80; ++i) {
            int sx = (i * 137 + 23) % W_;
            int sy = (i * 97  + 11) % H_;
            float alpha = 80.f + 60.f * std::sin((float)hud.anim_tick * 0.04f + i);
            p.setPen(QColor(200, 200, 220, (int)alpha));
            p.drawPoint(sx, sy);
        }

        int bw = 560, bh = 380;
        int bx = (W_-bw)/2, by = (H_-bh)/2;

        bool blink = (hud.anim_tick/14) % 2 == 0;
        panel(p, bx, by, bw, bh,
              blink ? QColor(0,180,220) : QColor(0,70,160), QColor(8,10,32));

        p.setPen(QPen(QColor(0,80,140), 1));
        p.drawLine(bx+10, by+28, bx+bw-10, by+28);

        p.setFont(QFont("Consolas", 30, QFont::Bold));
        p.setPen(QColor(0,220,255));
        p.drawText(QRect(bx, by+34, bw, 50), Qt::AlignCenter,
                   u8"\u25C6  VOID QUEST  \u25C6");

        p.setFont(QFont("Consolas", 11));
        p.setPen(QColor(90,130,170));
        p.drawText(QRect(bx, by+82, bw, 24), Qt::AlignCenter,
                   "3D MMO Roleplaying Game");

        // 3 menu items
        const char* items[3] = {
            "   Play  \u2014  Connect to Server",
            "   Singleplayer",
            "   Quit"
        };
        for (int i = 0; i < 3; ++i) {
            bool sel = (i == hud.menu_sel);
            QRect ir(bx+60, by+148+i*56, bw-120, 44);
            p.fillRect(ir, sel ? QColor(18,42,72) : QColor(12,12,26));
            p.setPen(QPen(sel ? QColor(0,200,180) : QColor(50,60,90), sel ? 2 : 1));
            p.drawRect(ir);
            p.setFont(QFont("Consolas", 14, sel ? QFont::Bold : QFont::Normal));
            p.setPen(sel ? QColor(0,255,200) : QColor(150,150,190));
            QString label = (sel ? u8"\u25B6 " : "  ") + QString::fromUtf8(items[i]);
            p.drawText(ir.adjusted(10,0,0,0), Qt::AlignVCenter|Qt::AlignLeft, label);
        }

        p.setFont(QFont("Consolas", 8));
        p.setPen(QColor(60,80,100));
        p.drawText(QRect(bx, by+bh-26, bw, 18), Qt::AlignCenter,
                   "[Up/Down or j/k]  Navigate    [Enter]  Select    [Q]  Quit");
    }

    void drawConnect(QPainter& p, const HudData& hud) {
        p.fillRect(0,0,W_,H_, QColor(5,5,18));
        int bw=480,bh=200, bx=(W_-bw)/2, by=(H_-bh)/2;
        panel(p, bx, by, bw, bh, QColor(0,160,200), QColor(10,10,22),
              "  Connect to Server");

        auto field = [&](int row, const QString& lbl, const QString& val, bool active){
            p.setFont(QFont("Consolas", 11));
            p.setPen(active ? QColor(0,220,220) : QColor(110,110,130));
            p.drawText(bx+18, by+row, lbl);
            QRect fr(bx+140, by+row-18, bw-160, 24);
            p.fillRect(fr, active ? QColor(18,28,50) : QColor(12,12,22));
            p.setPen(active ? QColor(220,220,220) : QColor(150,150,160));
            p.drawText(fr.adjusted(6,0,0,0), Qt::AlignVCenter,
                       val + (active ? "_" : ""));
        };
        field(76, "Host :", rs(hud.conn_host), hud.conn_cursor==0);
        field(116, "Port :", rs(hud.conn_port), hud.conn_cursor==1);

        p.setFont(QFont("Consolas", 9));
        p.setPen(QColor(70,90,110));
        p.drawText(bx+12, by+bh-14, "[Tab] Switch    [Enter] Connect    [Esc] Back");
    }

    void drawLogin(QPainter& p, const HudData& hud) {
        p.fillRect(0,0,W_,H_, QColor(5,5,18));
        int bw=500,bh=240, bx=(W_-bw)/2, by=(H_-bh)/2;
        QColor bord = hud.login_is_register ? QColor(200,140,0) : QColor(0,160,200);
        QString title = hud.login_is_register ? "  Register Account" : "  Login";
        panel(p, bx, by, bw, bh, bord, QColor(10,10,22), title);

        auto field = [&](int row, const QString& lbl, const QString& val,
                          bool active, bool mask){
            p.setFont(QFont("Consolas", 11));
            p.setPen(active ? QColor(0,220,220) : QColor(110,110,130));
            p.drawText(bx+18, by+row, lbl);
            QRect fr(bx+158, by+row-18, bw-178, 24);
            p.fillRect(fr, active ? QColor(18,28,50) : QColor(12,12,22));
            QString shown = mask ? QString(val.size(), QChar('*')) : val;
            p.setPen(active ? QColor(220,220,220) : QColor(150,150,160));
            p.drawText(fr.adjusted(6,0,0,0), Qt::AlignVCenter,
                       shown + (active ? "_" : ""));
        };
        QString user = rs(hud.login_user);
        QString pass(hud.login_pass_len, '*');
        field(78, "Username :", user, hud.login_cursor==0, false);
        field(118, "Password :", pass, hud.login_cursor==1, true);

        p.setFont(QFont("Consolas", 9));
        p.setPen(QColor(70,90,110));
        p.drawText(bx+12, by+162, "[Tab] Switch    [Enter] Submit    [Esc] Back");
        p.setPen(hud.login_is_register ? QColor(200,160,60) : QColor(60,160,200));
        p.drawText(bx+12, by+190,
                   hud.login_is_register ? "[F1]  Switch to Login"
                                         : "[F1]  Switch to Register");
    }

    void drawCharCreate(QPainter& p, const HudData& hud) {
        p.fillRect(0,0,W_,H_, QColor(5,5,18));
        int bw=620,bh=460, bx=(W_-bw)/2, by=(H_-bh)/2;
        QString title = hud.char_is_sp
            ? "  Create Character  (Singleplayer)"
            : "  Create Character";
        panel(p, bx, by, bw, bh, QColor(0,180,220), QColor(8,8,22), title);

        auto fieldRow = [&](int row, const QString& lbl, const QString& val,
                             bool active, QColor valCol = QColor(200,200,200)){
            p.setFont(QFont("Consolas", 11));
            p.setPen(active ? QColor(0,220,220) : QColor(100,110,130));
            p.drawText(bx+18, by+row, lbl);
            QRect vr(bx+148, by+row-18, bw-170, 24);
            p.fillRect(vr, active ? QColor(18,28,54) : QColor(12,12,26));
            p.setPen(active ? valCol : valCol.darker(120));
            p.drawText(vr.adjusted(8,0,0,0), Qt::AlignVCenter, val);
        };

        bool c0=hud.char_cursor==0, c1=hud.char_cursor==1;
        bool c2=hud.char_cursor==2, c3=hud.char_cursor==3;

        fieldRow(68,  "Name   :", rs(hud.char_name) + (c0 ? "_" : ""), c0);
        QString clsVal = hud.char_n_classes > 0
            ? u8"\u25C4 " + rs(hud.char_class_name) + u8" \u25BA"
            : "(none — loading…)";
        fieldRow(108, "Class  :", clsVal, c1, QColor(220,220,100));
        fieldRow(148, "Symbol :", u8"\u25C4 " + rs(hud.char_symbol) + u8" \u25BA",
                 c2, QColor(220,220,100));

        {
            p.setFont(QFont("Consolas", 11));
            p.setPen(c3 ? QColor(0,220,220) : QColor(100,110,130));
            p.drawText(bx+18, by+188, "Color  :");
            QRect vr(bx+148, by+170, bw-170, 24);
            p.fillRect(vr, c3 ? QColor(18,28,54) : QColor(12,12,26));
            QColor col(hud.char_col_r, hud.char_col_g, hud.char_col_b);
            p.fillRect(bx+152, by+174, 20, 16, col);
            p.setPen(col);
            p.drawText(vr.adjusted(30,0,0,0), Qt::AlignVCenter,
                       c3 ? u8"\u25C4  Color  \u25BA" : "Color");
        }

        p.setPen(QColor(40,60,90));
        p.drawLine(bx+10, by+218, bx+bw-10, by+218);

        p.setFont(QFont("Consolas", 9));
        p.setPen(QColor(150,150,120));
        p.drawText(QRect(bx+12, by+226, bw-24, 80), Qt::TextWordWrap,
                   rs(hud.char_class_desc));

        p.setPen(QColor(60,80,100));
        p.drawText(bx+12, by+bh-16,
            "[Tab/Up/Down] Navigate    [Left/Right] Change    [Enter] Create    [Esc] Back");
    }

    void drawPlayingHUD(QPainter& p, const HudData& hud) {
        int gameW = W_ - RIGHT_W;

        // Camera mode label
        const char* camLabels[4] = {"TopDown","ThirdPerson","FirstPerson","2D-Pixel"};

        // ── Top bar ───────────────────────────────────────────────────────────
        p.fillRect(0, 0, W_, TOP_BAR, QColor(10,10,26,220));
        p.setPen(QPen(QColor(30,50,90), 1));
        p.drawLine(0, TOP_BAR, W_, TOP_BAR);
        p.setFont(QFont("Consolas", 11, QFont::Bold));
        p.setPen(QColor(0,200,220));

        // Show zone name in SP, world name in MP
        QString locLabel = hud.is_singleplayer
            ? rs(hud.zone_style)
            : rs(hud.world_name);

        QString topBar = QString(" VoidQuest  \u2502  %1  Lv.%2  \u2502  %3  \u2502  "
                                  "XP: %4/%5  \u2502  [%6]")
            .arg(rs(hud.player_name))
            .arg(hud.player_level)
            .arg(locLabel)
            .arg(hud.xp).arg(hud.xp_next)
            .arg(camLabels[hud.camera_mode]);
        p.drawText(6, TOP_BAR-8, topBar);

        // ── Right stats panel ─────────────────────────────────────────────────
        int px = gameW, py = TOP_BAR;
        int ph = H_ - TOP_BAR;
        p.fillRect(px, py, RIGHT_W, ph, QColor(7,9,22,220));
        p.setPen(QPen(QColor(40,60,100), 1));
        p.drawLine(px, py, px, py+ph);

        int ry = py + 16;
        auto stat = [&](const QString& s, QColor c, bool bold=false){
            p.setFont(QFont("Consolas", 10, bold ? QFont::Bold : QFont::Normal));
            p.setPen(c);
            p.drawText(px+8, ry, s);
            ry += 18;
        };

        stat(rs(hud.player_name), {0,220,255}, true);
        stat(QString("Class: %1   Lv.%2").arg(rs(hud.player_class)).arg(hud.player_level),
             {155,155,200});
        stat(QString("XP: %1 / %2").arg(hud.xp).arg(hud.xp_next), {150,175,115});
        if (hud.stat_points > 0)
            stat(QString(u8"\u2605 Stat Points: %1").arg(hud.stat_points), {220,200,60}, true);
        ry += 4;

        hpBar(p, px+8, ry, RIGHT_W-16, 19, "HP",
              hud.hp, hud.max_hp, QColor(180,38,38));  ry += 23;
        hpBar(p, px+8, ry, RIGHT_W-16, 19, "MP",
              hud.mp, hud.max_mp, QColor(38,78,200));  ry += 27;

        stat(u8"\u2500 Stats \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500",
             {70,90,110});
        stat(QString("STR:%1   DEX:%2").arg(hud.stat_str,3).arg(hud.stat_dex,3),
             {200,158,98});
        stat(QString("INT:%1   VIT:%2").arg(hud.stat_int,3).arg(hud.stat_vit,3),
             {98,158,200});
        stat(QString("ATK:%1   DEF:%2").arg(hud.atk,3).arg(hud.def_,3), {200,198,98});
        ry += 4;

        stat(u8"\u2500 Equipment \u2500\u2500\u2500\u2500\u2500\u2500\u2500", {70,90,110});

        auto eqRow = [&](const QString& slot, const rust::String& name,
                          uint8_t er, uint8_t eg, uint8_t eb){
            p.setFont(QFont("Consolas", 9));
            p.setPen(QColor(120,140,120));
            p.drawText(px+8, ry, slot + ":");
            p.setPen(name.empty() ? QColor(70,70,70) : QColor(er,eg,eb));
            p.drawText(px+58, ry,
                       name.empty() ? "(none)"
                                    : QString::fromStdString(std::string(name)));
            ry += 17;
        };
        eqRow("Wpn", hud.eq_weapon, hud.eq_weapon_r, hud.eq_weapon_g, hud.eq_weapon_b);
        eqRow("Arm", hud.eq_armor,  hud.eq_armor_r,  hud.eq_armor_g,  hud.eq_armor_b);
        eqRow("Hlm", hud.eq_helmet, hud.eq_helmet_r, hud.eq_helmet_g, hud.eq_helmet_b);
        eqRow("Rng", hud.eq_ring,   hud.eq_ring_r,   hud.eq_ring_g,   hud.eq_ring_b);
        ry += 4;

        // Nearby monsters
        {
            QJsonDocument jd = QJsonDocument::fromJson(
                QByteArray::fromStdString(std::string(hud.nearby_json)));
            if (jd.isArray() && !jd.array().isEmpty()) {
                stat(u8"\u2500 Nearby \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500",
                     {70,90,110});
                for (const QJsonValue& v : jd.array()) {
                    QJsonObject o = v.toObject();
                    QString n = o["name"].toString();
                    int hp = o["hp"].toInt(), mhp = o["max_hp"].toInt();
                    p.setFont(QFont("Consolas", 9));
                    p.setPen(QColor(180,120,120));
                    p.drawText(px+8, ry, QString("%1  %2/%3").arg(n).arg(hp).arg(mhp));
                    ry += 16;
                    if (ry > H_ - LOG_H - 30) break;
                }
            }
        }

        // Hints
        p.setFont(QFont("Consolas", 8));
        p.setPen(QColor(55,75,95));
        QString hints = hud.is_singleplayer
            ? "[i]Inv [c]Eq [e]Pick [WASD]Move [Z]Zone [F5]Cam"
            : "[i]Inv [c]Eq [t]Chat [e]Pick [WASD]Move [F5]Cam";
        p.drawText(px+6, H_-6, hints);

        // ── Log panel ─────────────────────────────────────────────────────────
        int logY = H_ - LOG_H;
        p.fillRect(0, logY, gameW, LOG_H, QColor(6,7,18,210));
        p.setPen(QPen(QColor(30,50,80), 1));
        p.drawLine(0, logY, gameW, logY);

        p.setFont(QFont("Consolas", 10, QFont::Bold));
        p.setPen(QColor(90,130,170));
        p.drawText(8, logY+14, "Messages");

        {
            QJsonDocument jd = QJsonDocument::fromJson(
                QByteArray::fromStdString(std::string(hud.log_json)));
            if (jd.isArray()) {
                QJsonArray arr = jd.array();
                int lineH = 14;
                int maxLines = (LOG_H - 26) / lineH;
                int start = std::max(0, (int)arr.size() - maxLines);
                int row = logY + 24;
                for (int i = start; i < (int)arr.size(); ++i, row += lineH) {
                    QString line = arr[i].toString();
                    QColor c;
                    if (line.startsWith("[ERROR]"))        c = QColor(220, 60, 60);
                    else if (line.startsWith("***"))       c = QColor(220,200, 60);
                    else if (line.startsWith("★"))         c = QColor(255,220, 80);
                    else if (line.startsWith("* "))        c = QColor(100,180,220);
                    else if (line.startsWith("<"))         c = QColor( 80,220,170);
                    else if (line.startsWith("(Chat"))     c = QColor(140,140,140);
                    else                                   c = QColor(170,170,170);
                    p.setFont(QFont("Consolas", 9));
                    p.setPen(c);
                    p.drawText(8, row, line);
                }
            }
        }

        // ── Chat bar ──────────────────────────────────────────────────────────
        if (hud.chat_active) {
            p.fillRect(0, H_-26, gameW, 26, QColor(14,14,38));
            p.setPen(QPen(QColor(0,140,190), 1));
            p.drawLine(0, H_-26, gameW, H_-26);
            p.setFont(QFont("Consolas", 11, QFont::Bold));
            p.setPen(QColor(0,200,220));
            p.drawText(8, H_-7, "Chat:");
            p.setPen(QColor(220,220,220));
            p.drawText(62, H_-7, rs(hud.chat_buf) + "_");
        }

        // ── Draggable windows ─────────────────────────────────────────────────
        if (hud.inv_open)   drawInventoryWin(p, hud);
        if (hud.equip_open) drawEquipWin(p, hud);
    }

    void drawInventoryWin(QPainter& p, const HudData& hud) {
        int bx = invWin_.x, by = invWin_.y;
        int bw = invWin_.w, bh = invWin_.h;
        panel(p, bx, by, bw, bh, QColor(80,100,160), QColor(12,12,28,230),
              "  Inventory  (drag title • resize corner)");

        QJsonDocument jd = QJsonDocument::fromJson(
            QByteArray::fromStdString(std::string(hud.inventory_json)));

        if (!jd.isArray() || jd.array().isEmpty()) {
            p.setFont(QFont("Consolas", 10));
            p.setPen(QColor(90,90,90));
            p.drawText(bx+18, by+42, "(empty)");
        } else {
            QJsonArray arr = jd.array();
            int lineH = 20;
            int maxItems = (bh - 56) / lineH;
            for (int i = 0; i < (int)arr.size() && i < maxItems; ++i) {
                QJsonObject item = arr[i].toObject();
                bool sel = (i == hud.inv_sel);
                QString name = item["name"].toString();
                QString sym  = item["symbol"].toString();
                QRect ir(bx+4, by+24+i*lineH, bw-8, lineH-2);
                if (sel) p.fillRect(ir, QColor(18,40,68));
                p.setFont(QFont("Consolas", 10, sel ? QFont::Bold : QFont::Normal));
                p.setPen(sel ? QColor(0,240,200) : QColor(170,170,170));
                p.drawText(ir.adjusted(8,2,0,0), Qt::AlignVCenter,
                           (sel ? u8"\u25B6 " : "  ") + sym + " " + name);
            }
        }

        // Resize grip
        p.setPen(QColor(60,80,120));
        for (int d = 4; d <= 12; d += 4) {
            p.drawLine(bx+bw-d, by+bh-2, bx+bw-2, by+bh-d);
        }

        p.setFont(QFont("Consolas", 8));
        p.setPen(QColor(70,90,110));
        p.drawText(bx+8, by+bh-10,
                   "[Up/Down] Sel   [e] Equip   [u] Use   [d] Drop   [i/Esc] Close");
    }

    void drawEquipWin(QPainter& p, const HudData& hud) {
        int bx = eqWin_.x, by = eqWin_.y;
        int bw = eqWin_.w, bh = eqWin_.h;
        panel(p, bx, by, bw, bh, QColor(80,140,80), QColor(12,12,28,230),
              "  Equipment  (drag title • resize corner)");

        int ry = by + 30;
        auto row = [&](const QString& slot, const rust::String& name,
                        uint8_t r, uint8_t g, uint8_t b){
            p.setFont(QFont("Consolas", 10));
            p.setPen(QColor(130,155,130));
            p.drawText(bx+14, ry, slot + " :");
            p.setPen(name.empty() ? QColor(70,70,70) : QColor(r,g,b));
            p.drawText(bx+90, ry,
                       name.empty() ? "(none)"
                                    : QString::fromStdString(std::string(name)));
            ry += 20;
        };
        row("Weapon",  hud.eq_weapon, hud.eq_weapon_r, hud.eq_weapon_g, hud.eq_weapon_b);
        row("Armor",   hud.eq_armor,  hud.eq_armor_r,  hud.eq_armor_g,  hud.eq_armor_b);
        row("Helmet",  hud.eq_helmet, hud.eq_helmet_r, hud.eq_helmet_g, hud.eq_helmet_b);
        row("Ring",    hud.eq_ring,   hud.eq_ring_r,   hud.eq_ring_g,   hud.eq_ring_b);

        // Resize grip
        p.setPen(QColor(60,100,60));
        for (int d = 4; d <= 12; d += 4)
            p.drawLine(bx+bw-d, by+bh-2, bx+bw-2, by+bh-d);

        p.setFont(QFont("Consolas", 8));
        p.setPen(QColor(70,100,70));
        p.drawText(bx+8, by+bh-10, "[c / Esc]  Close");
    }

    void drawError(QPainter& p, const QString& msg) {
        QFont f("Consolas", 13, QFont::Bold);
        QFontMetrics fm(f);
        int mw = fm.horizontalAdvance(msg) + 32;
        int mh = 40;
        int mx = (W_-mw)/2, my = (H_-mh)/2;
        p.fillRect(mx-3, my-3, mw+6, mh+6, QColor(28,4,4));
        p.setPen(QPen(QColor(170,28,28), 2));
        p.drawRect(mx, my, mw, mh);
        p.setFont(f);
        p.setPen(QColor(255,80,80));
        p.drawText(QRect(mx, my, mw, mh), Qt::AlignCenter, msg);
    }
};

// ── Entry-point called from Rust main() ──────────────────────────────────────

void run_app(rust::Box<GameApp> game) {
    QSurfaceFormat fmt;
    fmt.setVersion(3, 3);
    fmt.setProfile(QSurfaceFormat::CoreProfile);
    fmt.setDepthBufferSize(24);
    fmt.setSwapInterval(1);
    QSurfaceFormat::setDefaultFormat(fmt);

    int argc = 0;
    QApplication app(argc, nullptr);
    app.setApplicationName("VoidQuest");
    app.setStyle("Fusion");

    QPalette dark;
    dark.setColor(QPalette::Window,          QColor(18, 18, 28));
    dark.setColor(QPalette::WindowText,      QColor(210, 210, 220));
    dark.setColor(QPalette::Base,            QColor(10, 10, 20));
    dark.setColor(QPalette::AlternateBase,   QColor(22, 22, 36));
    dark.setColor(QPalette::Text,            QColor(210, 210, 220));
    dark.setColor(QPalette::Button,          QColor(30, 30, 50));
    dark.setColor(QPalette::ButtonText,      QColor(210, 210, 220));
    dark.setColor(QPalette::Highlight,       QColor(0, 140, 180));
    dark.setColor(QPalette::HighlightedText, Qt::white);
    app.setPalette(dark);

    VqWidget3D w(std::move(game));
    w.resize(1280, 800);
    w.show();
    app.exec();
}

} // namespace vq
