// vite.config.js
import { defineConfig } from "file:///D:/a10/aikjx/gitcode/%E7%AE%97%E5%AD%90%E7%BB%9F%E4%B8%80%E7%B3%BB%E7%BB%9F%E5%AE%8C%E6%95%B4%E5%B7%A5%E7%A8%8B%E5%8C%85%20(operator-unified-system.zip)/operator-unified-system/frontend/node_modules/vite/dist/node/index.js";
import vue from "file:///D:/a10/aikjx/gitcode/%E7%AE%97%E5%AD%90%E7%BB%9F%E4%B8%80%E7%B3%BB%E7%BB%9F%E5%AE%8C%E6%95%B4%E5%B7%A5%E7%A8%8B%E5%8C%85%20(operator-unified-system.zip)/operator-unified-system/frontend/node_modules/@vitejs/plugin-vue/dist/index.mjs";
import { fileURLToPath, URL } from "node:url";
var __vite_injected_original_import_meta_url = "file:///D:/a10/aikjx/gitcode/%E7%AE%97%E5%AD%90%E7%BB%9F%E4%B8%80%E7%B3%BB%E7%BB%9F%E5%AE%8C%E6%95%B4%E5%B7%A5%E7%A8%8B%E5%8C%85%20(operator-unified-system.zip)/operator-unified-system/frontend/vite.config.js";
var vite_config_default = defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", __vite_injected_original_import_meta_url))
    }
  },
  server: {
    host: "0.0.0.0",
    port: 5173,
    proxy: {
      "/api": {
        target: "http://localhost:3000",
        changeOrigin: true
      }
    }
  },
  build: {
    chunkSizeWarningLimit: 2e3,
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL("./index.html", __vite_injected_original_import_meta_url))
      },
      output: {
        // 第三方大依赖独立分包，避免全部塞进主包，减小首屏体积
        manualChunks(id) {
          if (id.includes("node_modules")) {
            if (id.includes("3d-force-graph") || id.includes("/three/")) return "vendor-3d";
            if (id.includes("echarts") || id.includes("zrender")) return "vendor-echarts";
            if (id.includes("element-plus") || id.includes("@element-plus")) return "vendor-element";
            if (id.includes("vue") || id.includes("vue-router")) return "vendor-vue";
            return "vendor";
          }
        }
      }
    }
  }
});
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcuanMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCJEOlxcXFxhMTBcXFxcYWlranhcXFxcZ2l0Y29kZVxcXFxcdTdCOTdcdTVCNTBcdTdFREZcdTRFMDBcdTdDRkJcdTdFREZcdTVCOENcdTY1NzRcdTVERTVcdTdBMEJcdTUzMDUgKG9wZXJhdG9yLXVuaWZpZWQtc3lzdGVtLnppcClcXFxcb3BlcmF0b3ItdW5pZmllZC1zeXN0ZW1cXFxcZnJvbnRlbmRcIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfZmlsZW5hbWUgPSBcIkQ6XFxcXGExMFxcXFxhaWtqeFxcXFxnaXRjb2RlXFxcXFx1N0I5N1x1NUI1MFx1N0VERlx1NEUwMFx1N0NGQlx1N0VERlx1NUI4Q1x1NjU3NFx1NURFNVx1N0EwQlx1NTMwNSAob3BlcmF0b3ItdW5pZmllZC1zeXN0ZW0uemlwKVxcXFxvcGVyYXRvci11bmlmaWVkLXN5c3RlbVxcXFxmcm9udGVuZFxcXFx2aXRlLmNvbmZpZy5qc1wiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9pbXBvcnRfbWV0YV91cmwgPSBcImZpbGU6Ly8vRDovYTEwL2Fpa2p4L2dpdGNvZGUvJUU3JUFFJTk3JUU1JUFEJTkwJUU3JUJCJTlGJUU0JUI4JTgwJUU3JUIzJUJCJUU3JUJCJTlGJUU1JUFFJThDJUU2JTk1JUI0JUU1JUI3JUE1JUU3JUE4JThCJUU1JThDJTg1JTIwKG9wZXJhdG9yLXVuaWZpZWQtc3lzdGVtLnppcCkvb3BlcmF0b3ItdW5pZmllZC1zeXN0ZW0vZnJvbnRlbmQvdml0ZS5jb25maWcuanNcIjtpbXBvcnQgeyBkZWZpbmVDb25maWcgfSBmcm9tICd2aXRlJ1xyXG5pbXBvcnQgdnVlIGZyb20gJ0B2aXRlanMvcGx1Z2luLXZ1ZSdcclxuaW1wb3J0IHsgZmlsZVVSTFRvUGF0aCwgVVJMIH0gZnJvbSAnbm9kZTp1cmwnXHJcblxyXG5leHBvcnQgZGVmYXVsdCBkZWZpbmVDb25maWcoe1xyXG4gIHBsdWdpbnM6IFt2dWUoKV0sXHJcbiAgcmVzb2x2ZToge1xyXG4gICAgYWxpYXM6IHtcclxuICAgICAgJ0AnOiBmaWxlVVJMVG9QYXRoKG5ldyBVUkwoJy4vc3JjJywgaW1wb3J0Lm1ldGEudXJsKSlcclxuICAgIH1cclxuICB9LFxyXG4gIHNlcnZlcjoge1xyXG4gICAgaG9zdDogJzAuMC4wLjAnLFxyXG4gICAgcG9ydDogNTE3MyxcclxuICAgIHByb3h5OiB7XHJcbiAgICAgICcvYXBpJzoge1xyXG4gICAgICAgIHRhcmdldDogJ2h0dHA6Ly9sb2NhbGhvc3Q6MzAwMCcsXHJcbiAgICAgICAgY2hhbmdlT3JpZ2luOiB0cnVlXHJcbiAgICAgIH1cclxuICAgIH1cclxuICB9LFxyXG4gIGJ1aWxkOiB7XHJcbiAgICBjaHVua1NpemVXYXJuaW5nTGltaXQ6IDIwMDAsXHJcbiAgICByb2xsdXBPcHRpb25zOiB7XHJcbiAgICAgIGlucHV0OiB7XHJcbiAgICAgICAgbWFpbjogZmlsZVVSTFRvUGF0aChuZXcgVVJMKCcuL2luZGV4Lmh0bWwnLCBpbXBvcnQubWV0YS51cmwpKVxyXG4gICAgICB9LFxyXG4gICAgICBvdXRwdXQ6IHtcclxuICAgICAgICAvLyBcdTdCMkNcdTRFMDlcdTY1QjlcdTU5MjdcdTRGOURcdThENTZcdTcyRUNcdTdBQ0JcdTUyMDZcdTUzMDVcdUZGMENcdTkwN0ZcdTUxNERcdTUxNjhcdTkwRThcdTU4NUVcdThGREJcdTRFM0JcdTUzMDVcdUZGMENcdTUxQ0ZcdTVDMEZcdTk5OTZcdTVDNEZcdTRGNTNcdTc5RUZcclxuICAgICAgICBtYW51YWxDaHVua3MoaWQpIHtcclxuICAgICAgICAgIGlmIChpZC5pbmNsdWRlcygnbm9kZV9tb2R1bGVzJykpIHtcclxuICAgICAgICAgICAgaWYgKGlkLmluY2x1ZGVzKCczZC1mb3JjZS1ncmFwaCcpIHx8IGlkLmluY2x1ZGVzKCcvdGhyZWUvJykpIHJldHVybiAndmVuZG9yLTNkJ1xyXG4gICAgICAgICAgICBpZiAoaWQuaW5jbHVkZXMoJ2VjaGFydHMnKSB8fCBpZC5pbmNsdWRlcygnenJlbmRlcicpKSByZXR1cm4gJ3ZlbmRvci1lY2hhcnRzJ1xyXG4gICAgICAgICAgICBpZiAoaWQuaW5jbHVkZXMoJ2VsZW1lbnQtcGx1cycpIHx8IGlkLmluY2x1ZGVzKCdAZWxlbWVudC1wbHVzJykpIHJldHVybiAndmVuZG9yLWVsZW1lbnQnXHJcbiAgICAgICAgICAgIGlmIChpZC5pbmNsdWRlcygndnVlJykgfHwgaWQuaW5jbHVkZXMoJ3Z1ZS1yb3V0ZXInKSkgcmV0dXJuICd2ZW5kb3ItdnVlJ1xyXG4gICAgICAgICAgICByZXR1cm4gJ3ZlbmRvcidcclxuICAgICAgICAgIH1cclxuICAgICAgICB9XHJcbiAgICAgIH1cclxuICAgIH1cclxuICB9XHJcbn0pXHJcbiJdLAogICJtYXBwaW5ncyI6ICI7QUFBdWpCLFNBQVMsb0JBQW9CO0FBQ3BsQixPQUFPLFNBQVM7QUFDaEIsU0FBUyxlQUFlLFdBQVc7QUFGZ1IsSUFBTSwyQ0FBMkM7QUFJcFcsSUFBTyxzQkFBUSxhQUFhO0FBQUEsRUFDMUIsU0FBUyxDQUFDLElBQUksQ0FBQztBQUFBLEVBQ2YsU0FBUztBQUFBLElBQ1AsT0FBTztBQUFBLE1BQ0wsS0FBSyxjQUFjLElBQUksSUFBSSxTQUFTLHdDQUFlLENBQUM7QUFBQSxJQUN0RDtBQUFBLEVBQ0Y7QUFBQSxFQUNBLFFBQVE7QUFBQSxJQUNOLE1BQU07QUFBQSxJQUNOLE1BQU07QUFBQSxJQUNOLE9BQU87QUFBQSxNQUNMLFFBQVE7QUFBQSxRQUNOLFFBQVE7QUFBQSxRQUNSLGNBQWM7QUFBQSxNQUNoQjtBQUFBLElBQ0Y7QUFBQSxFQUNGO0FBQUEsRUFDQSxPQUFPO0FBQUEsSUFDTCx1QkFBdUI7QUFBQSxJQUN2QixlQUFlO0FBQUEsTUFDYixPQUFPO0FBQUEsUUFDTCxNQUFNLGNBQWMsSUFBSSxJQUFJLGdCQUFnQix3Q0FBZSxDQUFDO0FBQUEsTUFDOUQ7QUFBQSxNQUNBLFFBQVE7QUFBQTtBQUFBLFFBRU4sYUFBYSxJQUFJO0FBQ2YsY0FBSSxHQUFHLFNBQVMsY0FBYyxHQUFHO0FBQy9CLGdCQUFJLEdBQUcsU0FBUyxnQkFBZ0IsS0FBSyxHQUFHLFNBQVMsU0FBUyxFQUFHLFFBQU87QUFDcEUsZ0JBQUksR0FBRyxTQUFTLFNBQVMsS0FBSyxHQUFHLFNBQVMsU0FBUyxFQUFHLFFBQU87QUFDN0QsZ0JBQUksR0FBRyxTQUFTLGNBQWMsS0FBSyxHQUFHLFNBQVMsZUFBZSxFQUFHLFFBQU87QUFDeEUsZ0JBQUksR0FBRyxTQUFTLEtBQUssS0FBSyxHQUFHLFNBQVMsWUFBWSxFQUFHLFFBQU87QUFDNUQsbUJBQU87QUFBQSxVQUNUO0FBQUEsUUFDRjtBQUFBLE1BQ0Y7QUFBQSxJQUNGO0FBQUEsRUFDRjtBQUNGLENBQUM7IiwKICAibmFtZXMiOiBbXQp9Cg==
