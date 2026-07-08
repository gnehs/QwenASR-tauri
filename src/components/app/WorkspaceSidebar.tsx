import {
  ListTodoIcon,
  Settings2Icon,
} from "lucide-react";
import type { ComponentType } from "react";

import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";
import type { WorkspaceView } from "@/types/transcription";

const navItems: Array<{
  id: WorkspaceView;
  label: string;
  icon: ComponentType;
}> = [
  {
    id: "tasks",
    label: "任務管理",
    icon: ListTodoIcon,
  },
];

export function WorkspaceSidebar({
  activeView,
  onViewChange,
}: {
  activeView: WorkspaceView;
  onViewChange: (view: WorkspaceView) => void;
}) {
  return (
    <Sidebar collapsible="icon" variant="sidebar">
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>工作流程</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.map((item) => {
                const Icon = item.icon;
                return (
                  <SidebarMenuItem key={item.id}>
                    <SidebarMenuButton
                      isActive={activeView === item.id}
                      tooltip={item.label}
                      onClick={() => onViewChange(item.id)}
                    >
                      <Icon />
                      <span>{item.label}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              isActive={activeView === "settings"}
              tooltip="設定"
              onClick={() => onViewChange("settings")}
            >
              <Settings2Icon />
              <span>設定</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
