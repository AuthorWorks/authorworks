import React from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft, Moon, Type, Save, Database } from 'lucide-react';
import { Card } from '../components/Card';

export const Settings: React.FC = () => {
  const navigate = useNavigate();

  return (
    <div className="min-h-screen p-4 pb-24">
      <div className="flex items-center gap-3 mb-6">
        <button
          onClick={() => navigate(-1)}
          className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
        >
          <ArrowLeft className="h-5 w-5" />
        </button>
        <h1 className="text-2xl font-playfair font-bold">Settings</h1>
      </div>

      <div className="space-y-6">
        {/* Appearance */}
        <div>
          <h2 className="text-lg font-semibold mb-3">Appearance</h2>
          <Card>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <Moon className="h-5 w-5 text-indigo-400" />
                <div>
                  <p className="font-medium">Dark Mode</p>
                  <p className="text-sm text-slate-400">Always enabled for optimal writing</p>
                </div>
              </div>
              <div className="h-6 w-11 bg-indigo-500 rounded-full flex items-center justify-end px-1">
                <div className="h-4 w-4 bg-white rounded-full"></div>
              </div>
            </div>
          </Card>
        </div>

        {/* Editor */}
        <div>
          <h2 className="text-lg font-semibold mb-3">Editor</h2>
          <Card className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <Type className="h-5 w-5 text-purple-400" />
                <div>
                  <p className="font-medium">Font Size</p>
                  <p className="text-sm text-slate-400">Adjust editor text size</p>
                </div>
              </div>
              <select className="bg-slate-800 border border-slate-700 rounded-lg px-3 py-1 text-sm">
                <option>Small</option>
                <option selected>Medium</option>
                <option>Large</option>
              </select>
            </div>

            <div className="flex items-center justify-between pt-3 border-t border-slate-800">
              <div className="flex items-center gap-3">
                <Save className="h-5 w-5 text-green-400" />
                <div>
                  <p className="font-medium">Auto-save Interval</p>
                  <p className="text-sm text-slate-400">How often to save your work</p>
                </div>
              </div>
              <select className="bg-slate-800 border border-slate-700 rounded-lg px-3 py-1 text-sm">
                <option selected>2 seconds</option>
                <option>5 seconds</option>
                <option>10 seconds</option>
              </select>
            </div>
          </Card>
        </div>

        {/* Data */}
        <div>
          <h2 className="text-lg font-semibold mb-3">Data & Storage</h2>
          <Card>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <Database className="h-5 w-5 text-pink-400" />
                <div>
                  <p className="font-medium">Clear Cache</p>
                  <p className="text-sm text-slate-400">Free up local storage space</p>
                </div>
              </div>
              <button className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg text-sm hover:bg-slate-700">
                Clear
              </button>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
};
